use std::{iter::Peekable, time::Duration};

use thiserror::Error;

#[cfg(test)]
mod test;

#[derive(Error, Debug)]
pub enum Error {
    #[error("missing file magic")]
    NoMagic,

    #[error("bad file header")]
    BadHeader,

    #[error("unexpected end-of-file")]
    UnexpectedEof,
}

#[derive(Debug, Clone)]
pub struct File {
    pub description: Option<String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Cue(Cue),
}

#[derive(Default, Debug, Clone)]
pub struct Cue {
    pub start: Duration,
    pub end: Duration,
    pub id: String,
    pub text: String,
    pub settings: CueSettings,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct CueSettings {
    pub region: Option<String>,
    pub writing_direction: WritingDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum WritingDirection {
    /// horizontal (a line extends horizontally and is offset vertically from
    /// the video viewport’s top edge, with consecutive lines displayed below
    /// each other)
    #[default]
    Horizontal,
    /// vertical growing left (a line extends vertically and is offset
    /// horizontally from the video viewport’s right edge, with consecutive
    /// lines displayed to the left of each other)
    VerticalLeft,
    /// vertical growing right (a line extends vertically and is offset
    /// horizontally from the video viewport’s left edge, with consecutive lines
    /// displayed to the right of each other)
    VerticalRight,
}

struct FileContext {
    seen_cue: bool,
    in_header: bool,
}

pub fn parse_file(input: &str) -> Result<File, Error> {
    use Error::*;

    let mut lines = input.split('\n').enumerate().peekable();

    let (_, line) = lines.next().ok_or(NoMagic)?;
    let line = expect_str(line, "WEBVTT", NoMagic)?;

    let description = if line.len() > 0 {
        let line = expect_char(line, &[' ', '\t'], BadHeader)?;
        Some(line.to_owned())
    } else {
        None
    };

    skip_blank_lines(&mut lines);

    let mut file_ctx = FileContext {
        in_header: false,
        seen_cue: false,
    };

    let mut blocks = vec![];

    while lines.peek().is_some() {
        if let Some(block) = parse_block(&mut lines, &mut file_ctx) {
            blocks.push(block);
        }

        skip_blank_lines(&mut lines);
    }

    Ok(File {
        description,
        blocks,
    })
}

struct BlockContext {
    line_count: usize,
    seen_eof: bool,
    seen_arrow: bool,
    cue: Option<Cue>,
    buffer: String,
}

fn parse_block<'a, I: Iterator<Item = (usize, &'a str)>>(
    lines: &mut Peekable<I>,
    file_ctx: &mut FileContext,
) -> Option<Block> {
    let mut block_ctx = BlockContext {
        line_count: 0,
        seen_arrow: false,
        seen_eof: false,

        cue: None,
        buffer: String::new(),
    };

    while let Some((_, line)) = lines.next() {
        block_ctx.line_count += 1;
        block_ctx.seen_eof = lines.peek().is_none();

        if line.contains("-->") {
            if !file_ctx.in_header {
                if (block_ctx.line_count == 1)
                    || (block_ctx.line_count == 2 && !block_ctx.seen_arrow)
                {
                    block_ctx.seen_arrow = true;

                    if let Some((start, end, settings)) = parse_cue_timings_settings(line) {
                        let buffer = std::mem::replace(&mut block_ctx.buffer, String::new());

                        let cue = Cue {
                            id: buffer,
                            start,
                            end,
                            settings,
                            ..Default::default()
                        };

                        block_ctx.cue = Some(cue);
                    }
                }
            }
        } else if line.is_empty() {
            break;
        } else {
            if !file_ctx.in_header && block_ctx.line_count == 2 {
                if !file_ctx.seen_cue {
                    if block_ctx.buffer.starts_with("STYLE") {
                        unimplemented!("WebVTT styles are unimplemented")
                    }

                    if block_ctx.buffer.starts_with("REGION") {
                        unimplemented!("WebVTT regions are unimplemented")
                    }
                }
            }

            if !block_ctx.buffer.is_empty() {
                block_ctx.buffer.push('\n');
            }

            block_ctx.buffer.push_str(line);
        }
    }

    if let Some(mut cue) = block_ctx.cue {
        cue.text = block_ctx.buffer;
        Some(Block::Cue(cue))
    } else {
        None
    }
}

fn parse_cue_timings_settings(line: &str) -> Option<(Duration, Duration, CueSettings)> {
    let line = line.trim_start();
    let (start_time, line) = parse_timestamp(line)?;

    let line = line.trim_start();
    let line = line.strip_prefix("-->")?;
    let line = line.trim_start();

    let (end_time, line) = parse_timestamp(line)?;
    let settings = parse_settings(line);

    Some((start_time, end_time, settings))
}

fn parse_settings(line: &str) -> CueSettings {
    let mut settings = CueSettings {
        region: None,
        writing_direction: WritingDirection::Horizontal,
    };

    for setting in line.split(' ') {
        if let Some((key, value)) = setting.split_once(':') {
            if key == "" || value == "" {
                continue;
            }

            match key {
                "region" => {
                    settings.region = Some(value.to_owned());
                }
                "vertical" => match value {
                    "lr" => settings.writing_direction = WritingDirection::VerticalLeft,
                    "rl" => settings.writing_direction = WritingDirection::VerticalRight,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    // there are no vertical regions
    if settings.writing_direction != WritingDirection::Horizontal {
        settings.region = None;
    }

    settings
}

/// Parses a timestamp from the given string. Returns a Duration that represents
/// the timestamp's offset from the zero, and the remainder of the string after
/// skipping the timestamp.
fn parse_timestamp(line: &str) -> Option<(Duration, &str)> {
    let mut has_hours = false;
    let mut buf = String::new();
    let mut places: Vec<u64> = vec![];

    let (mut last_idx, mut last_char);
    let mut chars = line.char_indices();

    // parse digits if there are any
    loop {
        let (_, char) = chars.next()?;
        last_char = char;

        if char.is_ascii_digit() {
            buf.push(char);
        } else {
            break;
        }
    }

    // if there were no digits, or we hit something that isn't ':', error
    if buf.is_empty() || last_char != ':' {
        return None;
    }

    let num = buf.parse().unwrap();

    // this could either be the hours place or the minutes place
    places.push(num);

    if num > 59 || buf.len() != 2 {
        has_hours = true;
    }

    buf.clear();

    // parse out some digits again
    loop {
        let (_, char) = chars.next()?;
        last_char = char;

        if char.is_ascii_digit() {
            buf.push(char);
        } else {
            break;
        }
    }

    // if we didn't get a 2-digit number, error
    if buf.len() != 2 {
        return None;
    }

    let num = buf.parse().unwrap();

    if num > 59 {
        return None;
    }

    places.push(num);

    buf.clear();

    // if we have an hours place, or we hit another colon, parse out another
    // 2-digit number
    if has_hours || last_char == ':' {
        // parse out some digits again
        loop {
            let (_, char) = chars.next()?;
            last_char = char;

            if char.is_ascii_digit() {
                buf.push(char);
            } else {
                break;
            }
        }

        // if we didn't get a 2-digit number, error
        if buf.len() != 2 {
            return None;
        }

        let num = buf.parse().unwrap();

        if num > 59 {
            return None;
        }

        places.push(num);

        buf.clear();
    }

    // if we hit a decimal point, we have a fractional number of seconds
    if last_char != '.' {
        return None;
    }

    // parse out some digits again
    loop {
        if let Some((idx, char)) = chars.next() {
            last_idx = idx;

            if char.is_ascii_digit() {
                buf.push(char);
            } else {
                break;
            }
        } else {
            last_idx = line.len();
            break;
        }
    }

    // if we didn't get a 3-digit number, error
    if buf.len() != 3 {
        return None;
    }

    places.push(buf.parse().unwrap());

    Some((
        match &places[..] {
            &[hours, minutes, seconds, millis] => {
                Duration::from_millis(millis + seconds * 1000 + minutes * 60_000 + hours * 3600_000)
            }
            &[minutes, seconds, millis] => {
                Duration::from_millis(millis + seconds * 1000 + minutes * 60_000)
            }
            _ => unreachable!(),
        },
        &line[last_idx..],
    ))
}

fn skip_blank_lines<'a, I: Iterator<Item = (usize, &'a str)>>(lines: &mut Peekable<I>) {
    loop {
        match lines.peek() {
            Some((_, line)) => {
                if !line.is_empty() {
                    break;
                }

                lines.next();
            }
            None => break,
        }
    }
}

fn expect_str<'a>(input: &'a str, pattern: &str, error: Error) -> Result<&'a str, Error> {
    input.strip_prefix(pattern).ok_or(error)
}

fn expect_pred<'a, F: FnMut(char) -> bool>(
    input: &'a str,
    pattern: F,
    error: Error,
) -> Result<&'a str, Error> {
    input.strip_prefix(pattern).ok_or(error)
}

fn expect_char<'a>(input: &'a str, pattern: &[char], error: Error) -> Result<&'a str, Error> {
    input.strip_prefix(pattern).ok_or(error)
}
