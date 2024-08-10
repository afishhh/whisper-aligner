//! This aligner implementation is based on [WhisperTimeSync](https://github.com/EtienneAb3d/WhisperTimeSync) which itself seems to be based on a variation of the [Needleman-Wunsch](https://en.wikipedia.org/wiki/Needleman%E2%80%93Wunsch_algorithm) sequence alignment algorithm.

// [Needleman-Wunsch](https://en.wikipedia.org/wiki/Needleman%E2%80%93Wunsch_algorithm) sequence
// alignment algorithm but minimizing cost instead of maximizing score and with distinct gap costs
// for items.
pub fn align(
    a: usize,
    b: usize,
    mut gap_cost_for: impl FnMut(bool, usize) -> f64,
    mut pairwise_cost: impl FnMut(usize, usize) -> f64,
) -> Vec<(Option<usize>, Option<usize>)> {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Choice {
        Match,
        A,
        B,
    }

    let mut choices = vec![vec![Choice::A; b + 1]; a + 1];
    let mut costs = vec![vec![0.0; b + 1]; a + 1];

    for i in 1..=a {
        choices[i][0] = Choice::A;
        costs[i][0] = costs[i - 1][0] + gap_cost_for(false, i - 1);
    }

    for j in 1..=b {
        choices[0][j] = Choice::B;
        costs[0][j] = costs[0][j - 1] + gap_cost_for(true, j - 1);
    }

    for i in 1..=a {
        for j in 1..=b {
            let match_cost = costs[i - 1][j - 1] + pairwise_cost(i - 1, j - 1);
            let delete_cost = costs[i - 1][j] + (costs[i][0] - costs[i - 1][0]);
            let insert_cost = costs[i][j - 1] + (costs[0][j] - costs[0][j - 1]);

            // println!(
            //     "match {} delete {} insert {}",
            //     match_cost, delete_cost, insert_cost
            // );

            if match_cost <= delete_cost && match_cost <= insert_cost {
                choices[i][j] = Choice::Match;
                costs[i][j] = match_cost;
            } else if delete_cost < insert_cost {
                choices[i][j] = Choice::A;
                costs[i][j] = delete_cost;
            } else {
                choices[i][j] = Choice::B;
                costs[i][j] = insert_cost;
            }

            // println!("score[{i}][{j}] = {}", costs[i][j]);
            // println!("choices[{i}][{j}] = {:?}", choices[i][j]);
        }
    }

    // for row in costs.iter() {
    //     for e in row {
    //         print!("{:>7} ", format!("{e:.3}"));
    //     }
    //     println!();
    // }

    let mut alignment = vec![];

    let mut i = a;
    let mut j = b;
    while i > 0 || j > 0 {
        alignment.push(match choices[i][j] {
            Choice::Match => {
                i -= 1;
                j -= 1;
                (Some(i), Some(j))
            }
            Choice::A => {
                i -= 1;
                (Some(i), None)
            }
            Choice::B => {
                j -= 1;
                (None, Some(j))
            }
        })
    }

    alignment.reverse();

    alignment
}

fn katakana2hiragana(chr: char) -> char {
    let value = chr as u32;
    if (0x30A1..=0x30F4).contains(&value) {
        char::from_u32(value - 96).unwrap()
    } else {
        chr
    }
}

struct TokenInfo {
    text: String,
    normalized: String,
}

impl TokenInfo {
    fn new(text: String) -> Self {
        let lowercase_hiragana: String = text
            .chars()
            .flat_map(char::to_lowercase)
            .map(katakana2hiragana)
            .collect();
        TokenInfo {
            normalized: lowercase_hiragana,
            text,
        }
    }
}

fn pairwise_cost(i: usize, j: usize, al: &TokenInfo, bl: &TokenInfo) -> f64 {
    let a = &al.text;
    let b = &bl.text;
    let alower = &al.normalized;
    let blower = &bl.normalized;
    let pos_term = (i + j) as f64 * 0.00001;

    if a == b {
        return pos_term;
    } else if alower == blower {
        return 0.01 + pos_term;
    } else if alower.trim() == blower.trim() {
        return 0.02 + pos_term;
    }

    // [0.0, 1.0], based on the length difference between the strings
    let length_term = 2.0 * std::cmp::min(alower.len(), blower.len()) as f64
        / (alower.len() + blower.len()) as f64;

    if alower.starts_with(blower)
        || alower.ends_with(blower)
        || blower.starts_with(alower)
        || blower.ends_with(alower)
        || (alower.len() > 2
            && blower.len() > 2
            && (alower.contains(blower) || blower.contains(alower)))
    {
        return 1.0 - length_term + pos_term;
    }

    2.0 - length_term + pos_term
}

pub fn text_align(
    a: impl Iterator<Item = String>,
    b: impl Iterator<Item = String>,
) -> Vec<(Option<usize>, Option<usize>)> {
    let an = a.into_iter().map(TokenInfo::new).collect::<Vec<_>>();
    let bn = b.into_iter().map(TokenInfo::new).collect::<Vec<_>>();

    align(
        an.len(),
        bn.len(),
        |is_b, i| {
            if if is_b { &bn } else { &an }[i]
                .text
                .chars()
                .any(char::is_alphanumeric)
            {
                1.0
            } else {
                0.1
            }
        },
        |ai, bi| pairwise_cost(ai, bi, &an[ai], &bn[bi]) * 0.99,
    )
}
