use std::ffi::{CStr, CString};

use ffmpeg::*;

use super::{AVError, AVI32Ext, AVResult};

unsafe fn codecctx_to_option_string(ctx: &AVCodecContext, plural: bool) -> AVResult<CString> {
    let mut out: Vec<u8> = Vec::with_capacity(1024);
    let len =
        av_channel_layout_describe(&ctx.ch_layout, out.as_mut_ptr() as *mut i8, out.capacity())
            .av_usize()?;
    out.set_len(len);
    Ok(CString::new(format!(
        "sample_fmt{plural}={}:sample_rate{plural}={}:channel_layout{plural}={}",
        CStr::from_ptr(av_get_sample_fmt_name(ctx.sample_fmt))
            .to_str()
            .unwrap(),
        ctx.sample_rate,
        CString::from_vec_with_nul(out).unwrap().to_str().unwrap(),
        plural = if plural { "s" } else { "" }
    ))
    .unwrap())
}

pub unsafe fn audio_demux_transcode_16khz_pcmf32le(
    src: *mut AVIOContext,
) -> AVResult<impl Iterator<Item = AVResult<*mut AVFrame>>> {
    let mut fmtctx = avformat_alloc_context();
    (*fmtctx).pb = src;
    avformat_open_input(
        &mut fmtctx,
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null_mut(),
    )
    .av_void()?;

    let streams = std::slice::from_raw_parts((*fmtctx).streams, (*fmtctx).nb_streams as usize);
    let stream = streams
        .iter()
        .find(|s| (*(***s).codecpar).codec_type == AVMediaType::AVMEDIA_TYPE_AUDIO)
        .map(|s| &**s)
        .unwrap();

    let codec = avcodec_find_decoder((*stream.codecpar).codec_id);
    let codecctx = avcodec_alloc_context3(codec);
    avcodec_parameters_to_context(codecctx, stream.codecpar).av_void()?;
    avcodec_open2(codecctx, codec, std::ptr::null_mut()).av_void()?;

    let mut abufferctx = std::ptr::null_mut();
    let mut aformatctx = std::ptr::null_mut();
    let mut abuffersinkctx = std::ptr::null_mut();
    let abuffer = avfilter_get_by_name(c"abuffer".as_ptr());
    let aformat = avfilter_get_by_name(c"aformat".as_ptr());
    let abuffersink = avfilter_get_by_name(c"abuffersink".as_ptr());
    let filter_graph = avfilter_graph_alloc();

    avfilter_graph_create_filter(
        &mut abufferctx,
        abuffer,
        std::ptr::null(),
        codecctx_to_option_string(&*codecctx, false)?.as_ptr(),
        std::ptr::null_mut(),
        filter_graph,
    )
    .av_void()?;

    avfilter_graph_create_filter(
        &mut aformatctx,
        aformat,
        std::ptr::null(),
        c"sample_fmts=flt:sample_rates=16000:channel_layouts=mono".as_ptr(),
        std::ptr::null_mut(),
        filter_graph,
    )
    .av_void()?;

    avfilter_graph_create_filter(
        &mut abuffersinkctx,
        abuffersink,
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null_mut(),
        filter_graph,
    )
    .av_void()?;

    avfilter_link(abufferctx, 0, aformatctx, 0).av_void()?;
    avfilter_link(aformatctx, 0, abuffersinkctx, 0).av_void()?;

    avfilter_graph_config(filter_graph, std::ptr::null_mut()).av_void()?;

    let frame = av_frame_alloc();
    let packet = av_packet_alloc();

    let mut decoded_frames = std::iter::from_fn(move || loop {
        match avcodec_receive_frame(codecctx, frame).av_void() {
            Err(AVError(value)) if value == AVERROR(EAGAIN) => {
                if let Err(e) = match av_read_frame(fmtctx, packet).av_void() {
                    Err(AVError(AVERROR_EOF)) => {
                        avcodec_send_packet(codecctx, std::ptr::null_mut()).av_void()
                    }
                    other => other.and_then(|_| avcodec_send_packet(codecctx, packet).av_void()),
                } {
                    return Some(Err(e));
                }
            }
            Err(AVError(AVERROR_EOF)) => return None,
            other => return Some(other.map(|_| frame)),
        };
    });

    Ok(std::iter::from_fn(move || loop {
        match av_buffersink_get_frame(abuffersinkctx, frame).av_void() {
            Err(AVError(value)) if value == AVERROR(EAGAIN) => {
                if let Err(e) = decoded_frames
                    .next()
                    .unwrap_or(Ok(std::ptr::null_mut()))
                    .and_then(|frame| av_buffersrc_write_frame(abufferctx, frame).av_void())
                {
                    return Some(Err(e));
                }
            }
            Err(AVError(AVERROR_EOF)) => return None,
            other => return Some(other.map(|_| frame)),
        }
    }))
}
