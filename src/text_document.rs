use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

#[derive(Debug)]
pub struct FullTextDocument {
    language_id: String,
    version: i32,
    content: String,
    line_offsets: Vec<u32>,
}

fn computed_line_offsets(text: &str, is_at_line_start: bool, text_offset: Option<u32>) -> Vec<u32> {
    let text_offset = text_offset.unwrap_or(0);
    let mut line_offsets = if is_at_line_start {
        vec![text_offset]
    } else {
        vec![]
    };

    let mut chars = text.char_indices().peekable();
    while let Some((idx, char)) = chars.next() {
        let idx: u32 = idx
            .try_into()
            .expect("The length of the text involved in the calculation is too long");
        if char == '\r' && chars.peek() == Some(&(idx as usize + 1, '\n')) {
            chars.next();
            line_offsets.push(text_offset + idx + 2);
        } else if char == '\n' || char == '\r' {
            line_offsets.push(text_offset + idx + 1);
        }
    }

    line_offsets
}

/// given a string (in UTF-8) and a byte offset, returns the offset in UTF-16 code units
///
/// for example, consider a string containing a single 4-byte emoji. 4-byte characters
/// in UTF-8 are supplementary plane characters that require two UTF-16 code units
/// (surrogate pairs).
///
/// in this example:
/// - offset 4 returns 2;
/// - offsets 1, 2 or 3 return 0, because they are not on a character boundary and round down;
/// - offset 5+ will return 2, the length of the string in UTF-16
fn line_offset_utf16(line: &str, offset: u32) -> u32 {
    let mut c = 0;
    for (idx, char) in line.char_indices() {
        if idx + char.len_utf8() > offset as usize || idx == offset as usize {
            break;
        }
        c += char.len_utf16() as u32;
    }
    c
}

impl FullTextDocument {
    pub fn new(language_id: String, version: i32, content: String) -> Self {
        let line_offsets = computed_line_offsets(&content, true, None);
        Self {
            language_id,
            version,
            content,
            line_offsets,
        }
    }

    pub fn update(&mut self, changes: &[TextDocumentContentChangeEvent], version: i32) {
        for change in changes {
            let TextDocumentContentChangeEvent { range, text, .. } = change;
            match range {
                Some(range) => {
                    // update content
                    let Range { start, end } = range;
                    let (start_offset, end_offset) = (self.offset_at(*start), self.offset_at(*end));
                    assert!(
                        start_offset <= end_offset,
                        "Start offset must be less than end offset. {}:{} (offset {}) is not <= {}:{} (offset {})",
                        start.line, start.character, start_offset,
                        end.line, end.character, end_offset
                    );
                    let (start_slice, end_slice) = (
                        self.content.get(0..start_offset as usize).unwrap_or(""),
                        self.content.get(end_offset as usize..).unwrap_or(""),
                    );
                    self.content = start_slice
                        .chars()
                        .chain(text.chars())
                        .chain(end_slice.chars())
                        .collect();

                    let (start_line, end_line) = (start.line, end.line);
                    assert!(start_line <= end_line);
                    let added_line_offsets = computed_line_offsets(text, false, Some(start_offset));

                    self.line_offsets = self
                        .line_offsets
                        .as_slice()
                        .get(0..(start_line + 1) as usize)
                        .unwrap_or(&[])
                        .iter()
                        .chain(added_line_offsets.iter())
                        .chain(
                            self.line_offsets
                                .as_slice()
                                .get((end_line + 1) as usize..)
                                .unwrap_or(&[]),
                        )
                        .copied()
                        .collect::<Vec<_>>();

                    let diff =
                        (text.len() as i32).saturating_sub_unsigned(end_offset - start_offset);
                    if diff != 0 {
                        let (start, end) = (
                            start_line + 1 + added_line_offsets.len() as u32,
                            self.line_count(),
                        );
                        for i in start..end {
                            self.line_offsets[i as usize] =
                                self.line_offsets[i as usize].saturating_add_signed(diff);
                        }
                    }
                }
                None => {
                    // Full Text
                    // update line_offsets
                    self.line_offsets = computed_line_offsets(text, true, None);

                    // update content
                    self.content = text.to_owned();
                }
            }
        }

        self.version = version;
    }

    /// Document's language id
    pub fn language_id(&self) -> &str {
        &self.language_id
    }

    /// Document's version
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Get document content
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// use lsp_textdocument::FullTextDocument;
    /// use lsp_types::{Range, Position};
    ///
    /// let text_documents = FullTextDocument::new("plain_text".to_string(), 1, "hello rust!".to_string());
    ///
    /// // get document all content
    /// let content = text_documents.get_content(None);
    /// assert_eq!(content, "hello rust!");
    ///
    /// // get document specify content by range
    /// let (start, end) = (Position::new(0, 1), Position::new(0, 9));
    /// let range = Range::new(start, end);
    /// let sub_content = text_documents.get_content(Some(range));
    /// assert_eq!(sub_content, "ello rus");
    /// ```
    pub fn get_content(&self, range: Option<Range>) -> &str {
        match range {
            Some(Range { start, end }) => {
                let start = self.offset_at(start);
                let end = self.offset_at(end).min(self.content_len());
                self.content.get(start as usize..end as usize).unwrap()
            }
            None => &self.content,
        }
    }

    fn get_line_and_offset(&self, line: u32) -> Option<(&str, u32)> {
        self.line_offsets.get(line as usize).map(|&line_offset| {
            let len: u32 = self.content_len();
            let eol_offset = self.line_offsets.get((line + 1) as usize).unwrap_or(&len);
            let line = &self.content[line_offset as usize..*eol_offset as usize];
            (line, line_offset)
        })
    }

    fn get_line(&self, line: u32) -> Option<&str> {
        self.get_line_and_offset(line).map(|(line, _)| line)
    }

    /// A amount of document content line
    pub fn line_count(&self) -> u32 {
        self.line_offsets
            .len()
            .try_into()
            .expect("The number of lines of text passed in is too long")
    }

    /// The length of the document content in UTF-8 bytes
    pub fn content_len(&self) -> u32 {
        self.content
            .len()
            .try_into()
            .expect("The length of the text passed in is too long")
    }

    /// Converts a zero-based byte offset in the UTF8-encoded content to a position
    ///
    /// the offset is in bytes, the position is in UTF16 code units. rounds down if
    /// the offset is not on a code unit boundary, or is beyond the end of the
    /// content.
    pub fn position_at(&self, offset: u32) -> Position {
        let offset = offset.min(self.content_len());
        let line_count = self.line_count();
        if line_count == 1 {
            // only one line
            return Position {
                line: 0,
                character: line_offset_utf16(self.get_line(0).unwrap(), offset),
            };
        }

        let (mut low, mut high) = (0, line_count);
        while low < high {
            let mid = (low + high).div_floor(2);
            if offset
                > *self
                    .line_offsets
                    .get(mid as usize)
                    .expect("Unknown mid value")
            {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        if low == 0 {
            // offset is on the first line
            return Position {
                line: 0,
                character: line_offset_utf16(self.get_line(0).unwrap(), offset),
            };
        }

        let line = low - 1;

        Position {
            line,
            character: line_offset_utf16(
                self.get_line(line).unwrap(),
                offset - self.line_offsets[line as usize],
            ),
        }
    }

    /// Converts a position to a zero-based byte offset, suitable for slicing the
    /// UTF-8 encoded content.
    pub fn offset_at(&self, position: Position) -> u32 {
        let Position { line, character } = position;
        match self.get_line_and_offset(line) {
            Some((line, offset)) => {
                let mut c = 0;
                let iter = line.char_indices();
                for (idx, char) in iter {
                    if c == character {
                        return offset + idx as u32;
                    }
                    c += char.len_utf16() as u32;
                }
                offset + line.len() as u32
            }
            None => {
                if line >= self.line_count() {
                    self.content_len()
                } else {
                    0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_text_document() -> FullTextDocument {
        FullTextDocument::new(
            "js".to_string(),
            2,
            "he\nllo\nworld\r\nfoo\rbar".to_string(),
        )
    }

    #[test]
    fn test_offset_at() {
        let text_document = full_text_document();

        let offset = text_document.offset_at(Position {
            line: 1,
            character: 1,
        });
        assert_eq!(offset, 4);

        let offset = text_document.offset_at(Position {
            line: 2,
            character: 3,
        });
        assert_eq!(offset, 10);

        // the `f` in `foo` (\r\n is a single line terminator)
        let offset = text_document.offset_at(Position {
            line: 3,
            character: 1,
        });
        assert_eq!(offset, 15);
    }

    /// basic multilingual plane
    #[test]
    fn test_offset_at_bmp() {
        // Euro symbol
        let text_document = FullTextDocument::new("js".to_string(), 2, "\u{20AC} euro".to_string());

        let offset = text_document.offset_at(Position {
            line: 0,
            // E euro
            //   ^
            character: 2,
        });
        assert_eq!(offset, 4);
    }

    /// supplementary multilingual plane, aka surrogate pair
    #[test]
    fn test_offset_at_smp() {
        // Deseret Small Letter Yee
        let text_document = FullTextDocument::new("js".to_string(), 2, "\u{10437} yee".to_string());
        let offset = text_document.offset_at(Position {
            line: 0,
            // HL yee
            //    ^
            character: 3,
        });
        assert_eq!(offset, 5);
    }

    #[test]
    fn test_position_at() {
        let text_document = full_text_document();

        let position = text_document.position_at(5);
        assert_eq!(
            position,
            Position {
                line: 1,
                character: 2
            }
        );

        let position = text_document.position_at(11);
        assert_eq!(
            position,
            Position {
                line: 2,
                character: 4,
            }
        );

        let position = text_document.position_at(15);
        assert_eq!(
            position,
            Position {
                line: 3,
                character: 1,
            }
        );

        let position = text_document.position_at(0);
        assert_eq!(
            position,
            Position {
                line: 0,
                character: 0,
            }
        );
    }

    /// basic multilingual plane
    #[test]
    fn test_position_at_bmp() {
        // Euro symbol
        let text_document = FullTextDocument::new("js".to_string(), 2, "\u{20AC} euro".to_string());
        let position = text_document.position_at(4);
        assert_eq!(
            position,
            Position {
                line: 0,
                // E euro
                //   ^
                character: 2,
            }
        );

        // multi-line content
        let text_document =
            FullTextDocument::new("js".to_string(), 2, "\n\n\u{20AC} euro\n\n".to_string());
        let position = text_document.position_at(6);
        assert_eq!(
            position,
            Position {
                line: 2,
                // E euro
                //   ^
                character: 2,
            }
        );
    }

    /// supplementary multilingual plane, aka surrogate pair
    #[test]
    fn test_position_at_smp() {
        // Deseret Small Letter Yee
        let text_document = FullTextDocument::new("js".to_string(), 2, "\u{10437} yee".to_string());
        assert_eq!(
            text_document.position_at(5),
            Position {
                line: 0,
                // HL yee
                //    ^
                character: 3,
            }
        );

        // \u{10437} is 4 bytes wide. if not on a char boundary, round down
        assert_eq!(
            text_document.position_at(2),
            Position {
                line: 0,
                character: 0,
            }
        );

        // multi-line content
        let text_document =
            FullTextDocument::new("js".to_string(), 2, "\n\n\u{10437} yee\n\n".to_string());
        let position = text_document.position_at(7);
        assert_eq!(
            position,
            Position {
                line: 2,
                // HL yee
                //    ^
                character: 3,
            }
        );
    }

    #[test]
    fn test_get_content() {
        let text_document = full_text_document();

        let start = Position {
            line: 0,
            character: 0,
        };
        let end = Position {
            line: 1,
            character: 2,
        };
        let range = Range { start, end };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "he\nll");

        let end = Position {
            line: 100,
            character: 100,
        };
        let range = Range { start, end };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, text_document.content);

        let range = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 2,
                character: 3,
            },
        };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "llo\nwor");
    }

    /// basic multilingual plane
    #[test]
    fn test_get_content_bmp() {
        // Euro symbol
        let text_document = FullTextDocument::new("js".to_string(), 2, "\u{20AC} euro".to_string());

        // Euro symbol is 1 UTF16 code unit wide
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "\u{20AC}");

        // E euro
        //   ^
        let range = Range {
            start: Position {
                line: 0,
                character: 2,
            },
            end: Position {
                line: 0,
                character: 3,
            },
        };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "e");
    }

    /// supplementary multilingual plane, aka surrogate pairs
    #[test]
    fn test_get_content_smp() {
        // Deseret Small Letter Yee
        let text_document = FullTextDocument::new("js".to_string(), 2, "\u{10437} yee".to_string());

        // surrogate pairs are 2 UTF16 code units wide
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 2,
            },
        };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "\u{10437}");
    }

    #[test]
    fn test_update_full_content() {
        let mut text_document = full_text_document();
        let new_text = "hello\n js!";

        text_document.update(
            &[TextDocumentContentChangeEvent {
                text: new_text.to_string(),
                range: None,
                range_length: None,
            }],
            1,
        );

        assert_eq!(&text_document.content, new_text);
        assert_eq!(text_document.line_offsets, vec![0, 6]);
    }

    #[test]
    fn test_update_part_content() {
        let mut text_document = full_text_document();
        assert_eq!(text_document.version(), 2);
        let new_text = String::from("xx\ny");
        let range = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 3,
            },
        };
        text_document.update(
            &[TextDocumentContentChangeEvent {
                range: Some(range),
                range_length: None,
                text: new_text,
            }],
            1,
        );

        assert_eq!(&text_document.content, "he\nxx\ny\nworld\r\nfoo\rbar");
        assert_eq!(text_document.line_offsets, vec![0, 3, 6, 8, 15, 19]);
        assert_eq!(text_document.version(), 1)
    }

    #[test]
    #[should_panic(
        expected = "Start offset must be less than end offset. 2:0 (offset 7) is not <= 1:0 (offset 3)"
    )]
    fn test_update_invalid_range() {
        let mut text_document = full_text_document();
        // start is after end
        let range = Range {
            start: Position {
                line: 2,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 0,
            },
        };
        text_document.update(
            &[TextDocumentContentChangeEvent {
                text: String::from(""),
                range: Some(range),
                range_length: Some(0),
            }],
            1,
        );
    }
}
