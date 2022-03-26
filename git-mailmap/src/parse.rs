mod error {
    use bstr::BString;
    use quick_error::quick_error;

    quick_error! {
        #[derive(Debug)]
        pub enum Error {
            UnconsumedInput { line_number: usize, line: BString } {
                display("Line {} has too many names or emails, or none at all: {}", line_number, line)
            }
            Malformed { line_number: usize, line: BString, message: String } {
                display("{}: {:?}: {}", line_number, line, message)
            }
        }
    }
}

use crate::Entry;
use bstr::{BStr, ByteSlice};
pub use error::Error;

pub struct Lines<'a> {
    lines: bstr::Lines<'a>,
    line_no: usize,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Lines {
            lines: input.as_bstr().lines(),
            line_no: 0,
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = Result<Entry<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            self.line_no += 1;
            match line.first() {
                None => continue,
                Some(b) if *b == b'#' => continue,
                Some(_) => {}
            }
            let line = match line.as_bstr().find_not_byteset(BLANKS) {
                None => continue,
                Some(pos) => &line[pos..],
            };
            return parse_line(line.into(), self.line_no).into();
        }
        None
    }
}

const BLANKS: &[u8] = b" \t\r";

fn parse_line(line: &BStr, line_number: usize) -> Result<Entry<'_>, Error> {
    let (name1, email1, rest) = parse_name_and_email(line, line_number)?;
    let (name2, email2, rest) = parse_name_and_email(rest, line_number)?;
    if !trim(rest).is_empty() {
        return Err(Error::UnconsumedInput {
            line_number,
            line: line.into(),
        });
    }
    Ok(match (name1, email1, name2, email2) {
        (Some(proper_name), Some(commit_email), None, None) => Entry::change_name_by_email(proper_name, commit_email),
        (None, Some(proper_email), None, Some(commit_email)) => {
            Entry::change_email_by_email(proper_email, commit_email)
        }
        (Some(proper_name), Some(proper_email), None, Some(commit_email)) => {
            Entry::change_name_and_email_by_email(proper_name, proper_email, commit_email)
        }
        (Some(proper_name), Some(proper_email), Some(commit_name), Some(commit_email)) => {
            Entry::change_name_and_email_by_name_and_email(proper_name, proper_email, commit_name, commit_email)
        }
        _ => unreachable!("{:?}", line),
    })
}

fn parse_name_and_email(
    line: &BStr,
    line_number: usize,
) -> Result<(Option<&'_ BStr>, Option<&'_ BStr>, &'_ BStr), Error> {
    match line.find_byte(b'<') {
        Some(start_bracket) => {
            let email = &line[start_bracket + 1..];
            let closing_bracket = email.find_byte(b'>').ok_or_else(|| Error::Malformed {
                line_number,
                line: line.into(),
                message: "Missing closing bracket '>' in email".into(),
            })?;
            let email = trim(&email[..closing_bracket]);
            if email.is_empty() {
                return Err(Error::Malformed {
                    line_number,
                    line: line.into(),
                    message: "Email must not be empty".into(),
                });
            }
            let name = trim(&line[..start_bracket]);
            let rest = line[start_bracket + closing_bracket + 2..].as_bstr();
            Ok(((!name.is_empty()).then(|| name), Some(email), rest))
        }
        None => Ok((None, None, line)),
    }
}

fn trim(line: &[u8]) -> &BStr {
    // TODO: without str, use byteset find
    std::str::from_utf8(line).unwrap().trim().as_bytes().as_bstr()
}
