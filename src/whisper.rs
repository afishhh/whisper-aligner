use std::{
    borrow::Cow,
    ffi::{c_void, CStr, CString},
    fmt::Display,
    io::{Read, Write},
    num::{NonZero, NonZeroUsize},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::ff;

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s == 0.0 {
        let c = (l * 255.0) as u8;
        (c, c, c)
    } else {
        let hue_to_rgb = |p, q, mut t| -> f32 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }

            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 1.0 / 2.0 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        (
            (hue_to_rgb(p, q, h + 1. / 3.0) * 255.0) as u8,
            (hue_to_rgb(p, q, h) * 255.0) as u8,
            (hue_to_rgb(p, q, h - 1. / 3.0) * 255.0) as u8,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub probability: f32,
    pub start: i64,
    pub end: i64,
    pub text: String,
}

impl Token {
    fn write_colored(&self, mut out: impl std::io::Write) -> std::io::Result<()> {
        let (r, g, b) = hsl_to_rgb(self.probability * (100.0 / 360.0), 1.0, 0.5);
        write!(out, "\x1b[38;2;{r};{g};{b}m{}\x1b[0m", self.text)
    }
}

struct RawToken<'a> {
    #[allow(dead_code)]
    index: Option<i32>,
    data: whisper_cpp_sys::whisper_token_data,
    text: &'a CStr,
}

struct SimplerToken<'a> {
    data: whisper_cpp_sys::whisper_token_data,
    text: Cow<'a, str>,
}

unsafe fn fixup_whisper_tokens<'a>(
    tokens: impl IntoIterator<Item = RawToken<'a>>,
) -> Vec<SimplerToken<'a>> {
    let mut current_broken_probability_sum: f32 = 0.0;
    let mut current_broken_nmerge = 0;
    let mut current_broken_text = vec![];
    let mut current_broken_startts = 0;
    let mut current_broken_endts = 0;
    let mut current_broken_vlen = 0.0;

    let mut result = vec![];
    for token in tokens {
        if token.text.to_str().is_err() || !current_broken_text.is_empty() {
            if current_broken_text.is_empty() {
                current_broken_startts = token.data.t0;
                current_broken_endts = token.data.t1;
            }
            current_broken_text.extend_from_slice(token.text.to_bytes());
            current_broken_nmerge += 1;
            current_broken_probability_sum += token.data.p;
            current_broken_vlen += token.data.vlen;
        } else {
            result.push(SimplerToken {
                data: token.data,
                text: Cow::Borrowed(token.text.to_str().unwrap_unchecked()),
            })
        }

        if !current_broken_text.is_empty() && std::str::from_utf8(&current_broken_text).is_ok() {
            let text = String::from_utf8_unchecked(std::mem::take(&mut current_broken_text));
            println!("merged {current_broken_nmerge} partial tokens into \"{text}\"");
            result.push(SimplerToken {
                data: whisper_cpp_sys::whisper_token_data {
                    id: -1,
                    tid: -1,
                    p: current_broken_probability_sum / current_broken_nmerge as f32,
                    plog: -1.0,
                    pt: -1.0,
                    ptsum: -1.0,
                    t0: current_broken_startts,
                    t1: current_broken_endts,
                    t_dtw: -1,
                    vlen: current_broken_vlen,
                },
                text: Cow::Owned(text),
            });
            current_broken_vlen = 0.0;
            current_broken_probability_sum = 0.0;
            current_broken_nmerge = 0;
            current_broken_startts = 0;
            current_broken_endts = 0;
        }
    }

    if !current_broken_text.is_empty() {
        println!(
            "warning partial token left over in segment ({} bytes)",
            current_broken_text.len()
        )
    }

    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    pub language: String,
    pub segments: Vec<Vec<Token>>,
}

pub fn transcribe(file: impl Read, language: String, model: PathBuf) -> Transcription {
    let mut samples = vec![];

    unsafe {
        let frames = ff::audio_demux_transcode_16khz_pcmf32le(ff::read_to_avio(Box::new(file)))
            .unwrap()
            .map(|x| &*x.unwrap());
        for frame in frames {
            assert!(frame.format == ffmpeg::AVSampleFormat::AV_SAMPLE_FMT_FLT as i32);
            samples.extend_from_slice(std::slice::from_raw_parts(
                frame.data[0] as *const f32,
                frame.nb_samples as usize,
            ));
        }
    }

    let mut segments: Vec<Vec<Token>> = vec![];

    unsafe {
        use whisper_cpp_sys::*;
        let cparams = whisper_context_default_params();

        let model_cstr = CString::new(model.to_str().unwrap()).unwrap();
        let ctx = whisper_init_from_file_with_params(model_cstr.as_ptr(), cparams);

        let mut wparams =
            whisper_full_default_params(whisper_sampling_strategy_WHISPER_SAMPLING_BEAM_SEARCH);

        let language = CString::new(language.clone()).unwrap();
        wparams.language = language.as_ptr();
        wparams.translate = false;
        wparams.n_threads = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1) as i32;
        wparams.token_timestamps = true; // TODO: Figure out this whole "DTW whisper" thing
        wparams.no_timestamps = false;
        wparams.beam_search.beam_size = 5;

        unsafe extern "C" fn on_new_segment(
            ctx: *mut whisper_context,
            _whisper_state: *mut whisper_state,
            n_new: i32,
            user: *mut c_void,
        ) {
            let total = whisper_full_n_segments(ctx);
            for i in (total - n_new)..total {
                let fixed =
                    fixup_whisper_tokens((0..whisper_full_n_tokens(ctx, i)).map(|j| RawToken {
                        index: Some(j),
                        data: whisper_full_get_token_data(ctx, i, j),
                        text: CStr::from_ptr(whisper_full_get_token_text(ctx, i, j)),
                    }));

                let mut out = vec![];

                for SimplerToken { data, text } in fixed {
                    if text.starts_with("[_") && text.ends_with("]") {
                        continue;
                    }

                    let basic = Token {
                        start: data.t0,
                        end: data.t1,
                        probability: data.p,
                        text: text.to_string(),
                    };
                    basic.write_colored(&mut std::io::stdout());
                    out.push(basic);
                }

                (*(user as *mut Vec<Vec<Token>>)).push(out);
                println!()
            }
        }

        wparams.new_segment_callback = Some(on_new_segment);
        wparams.new_segment_callback_user_data =
            &mut segments as *mut Vec<Vec<Token>> as *mut c_void;

        whisper_full(ctx, wparams, samples.as_ptr(), samples.len() as i32);
    }

    Transcription { language, segments }
}
