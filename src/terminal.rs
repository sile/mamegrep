use std::{io::Write, time::Duration};

use orfail::OrFail;

use crate::{
    app::Event,
    canvas::{Frame, TokenPosition, TokenStyle},
};

#[derive(Debug)]
pub struct Terminal {
    size: TerminalSize,
    prev: Frame,
    show_cursor: bool,
}

impl Terminal {
    pub fn new() -> orfail::Result<Self> {
        // crossterm::execute!(
        //     std::io::stdout(),
        //     EnterAlternateScreen,
        //     crossterm::cursor::Hide,
        // )
        // .or_fail()?;
        // crossterm::terminal::enable_raw_mode().or_fail()?;

        // let (cols, rows) = crossterm::terminal::size().or_fail()?;
        // let size = TerminalSize {
        //     rows: rows as usize,
        //     cols: cols as usize,
        // };
        todo!()
        // Ok(Self {
        //     size,
        //     prev: Frame::new(size),
        //     show_cursor: false,
        // })
    }

    pub fn show_cursor(&mut self, position: TokenPosition) -> orfail::Result<()> {
        // if !self.show_cursor {
        //     crossterm::execute!(std::io::stdout(), crossterm::cursor::Show).or_fail()?;
        //     self.show_cursor = true;
        // }
        // crossterm::execute!(
        //     std::io::stdout(),
        //     crossterm::cursor::MoveTo(position.col as u16, position.row as u16)
        // )
        // .or_fail()?;
        Ok(())
    }

    pub fn hide_cursor(&mut self) -> orfail::Result<()> {
        // if self.show_cursor {
        //     crossterm::execute!(std::io::stdout(), crossterm::cursor::Hide).or_fail()?;
        //     self.show_cursor = false;
        // }
        Ok(())
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }

    pub fn next_event(&mut self) -> orfail::Result<Event> {
        // let timeout = Duration::from_secs(1);
        // while !crossterm::event::poll(timeout).or_fail()? {}

        // let event = crossterm::event::read().or_fail()?;
        // if let Event::Resize(cols, rows) = event {
        //     self.size.cols = cols as usize;
        //     self.size.rows = rows as usize;
        // }

        // Ok(event)
        todo!()
    }

    pub fn draw_frame(&mut self, frame: Frame) -> orfail::Result<()> {
        // let stdout = std::io::stdout();
        // let mut writer = stdout.lock();
        // for (row, line) in frame.dirty_lines(&self.prev) {
        //     crossterm::queue!(
        //         writer,
        //         crossterm::cursor::MoveTo(0, row as u16),
        //         crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        //     )
        //     .or_fail()?;

        //     for token in line.tokens() {
        //         let attributes = match token.style() {
        //             TokenStyle::Plain => Attributes::none(),
        //             TokenStyle::Bold => Attributes::none().with(Attribute::Bold),
        //             TokenStyle::Dim => Attributes::none().with(Attribute::Dim),
        //             TokenStyle::Underlined => Attributes::none().with(Attribute::Underlined),
        //             TokenStyle::Reverse => Attributes::none().with(Attribute::Reverse),
        //         };
        //         let content = StyledContent::new(
        //             ContentStyle {
        //                 attributes,
        //                 ..Default::default()
        //             },
        //             token.text(),
        //         );
        //         crossterm::queue!(writer, crossterm::style::PrintStyledContent(content))
        //             .or_fail()?;
        //     }
        // }

        // writer.flush().or_fail()?;
        // self.prev = frame;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // TODO:
        // let _ = crossterm::terminal::disable_raw_mode();
        // let _ = crossterm::execute!(
        //     std::io::stdout(),
        //     LeaveAlternateScreen,
        //     crossterm::cursor::Show,
        // );
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
}

impl TerminalSize {
    pub const EMPTY: Self = Self { cols: 0, rows: 0 };

    pub fn is_empty(self) -> bool {
        self.rows == 0 || self.cols == 0
    }
}
