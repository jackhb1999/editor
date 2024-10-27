use std::ffi;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use iced::{executor, application, Task, Element, Length, Settings, Theme, highlighter, Font};

use iced::widget::{container, row, text, column, horizontal_space, button, pick_list, tooltip};

use iced::widget::text_editor;
use iced::widget::tooltip::Position;
use crate::Message::Edit;

fn main() -> iced::Result {
    application(Editor::title, Editor::update, Editor::view)
        .theme(Editor::theme)
        .font(include_bytes!("../icon_fonts/fontello.ttf").as_slice())
        .run_with(Editor::new)
}

struct Editor {
    content: text_editor::Content,
    error: Option<Error>,
    path: Option<PathBuf>,
    theme: highlighter::Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    Open,
    New,
    Save,
    FileSaved(Result<PathBuf, Error>),
    ThemeSeleceted(highlighter::Theme),
}

fn icon<'a, Message>(uncode_point: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("fontello");
    text(uncode_point).font(ICON_FONT).into()
}

fn new_icon<'a, Message>() -> Element<'a, Message> {
    icon('\u{E831}')
}

fn save_icon<'a, Message>() -> Element<'a, Message> {
    icon('\u{E800}')
}

fn button_tooltip<'a>(content: Element<'a, Message>, label: &'a str, on_press: Message) -> Element<'a, Message> {
    tooltip(button(container(content).center_x(20))
                .padding([5,6])
                .on_press(on_press), label, Position::FollowCursor)
        .style(container::rounded_box)
        .into()
}


impl Editor {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                error: None,
                path: None,
                theme: highlighter::Theme::SolarizedDark,
            }, Task::perform(load_file(default_load_file()), Message::FileOpened)
        )
    }

    fn title(&self) -> String {
        String::from("This is a text editor.")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
                Task::none()
            }
            Message::FileOpened(Ok((path, content))) => {
                self.content = text_editor::Content::with_text(&content);
                self.path = Some(path);
                Task::none()
            }
            Message::FileOpened(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::Open => {
                Task::perform(pick_file(), Message::FileOpened)
            }
            Message::New => {
                self.content = text_editor::Content::new();
                self.path = None;
                Task::none()
            }
            Message::Save => {
                let content = self.content.text();
                Task::perform(save_file(self.path.clone(), content), Message::FileSaved)
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                Task::none()
            }
            Message::FileSaved(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::ThemeSeleceted(theme) => {
                self.theme = theme;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            button("Open").on_press(Message::Open),
            button(new_icon()).on_press(Message::New),
            button_tooltip(save_icon(),"Save File",Message::Save),
            horizontal_space(),
            pick_list(highlighter::Theme::ALL,Some(self.theme),Message::ThemeSeleceted)
        ].spacing(10);
        let input_content = text_editor(&self.content)
            .on_action(Message::Edit)
            .height(Length::Fill)
            .highlight(self.path.as_deref()
                           .and_then(Path::extension)
                           .and_then(ffi::OsStr::to_str)
                           .unwrap_or("rs"),
                       self.theme);
        let position = {
            let (line, column) = &self.content.cursor_position();
            text(format!("{}:{}", line + 1, column + 1))
        };
        let file_path = if let Some(Error::IOFailed(error)) = self.error.as_ref() {
            text(error.to_string())
        } else {
            match self.path.as_deref().and_then(Path::to_str) {
                None => {
                    text("New File")
                }
                Some(path) => {
                    text(path).size(15)
                }
            }
        };
        let status_bar = row![file_path,horizontal_space(),position];
        container(column![controls,input_content,status_bar]).padding(5).into()
    }

    fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}


// 自定义错误类型
#[derive(Debug, Clone)]
enum Error {
    IOFailed(ErrorKind),
    DialogClosed,
}

// &str String PathBuf...
async fn load_file(path: impl AsRef<Path>) -> Result<(PathBuf, Arc<String>), Error> {
    let content = tokio::fs::read_to_string(path.as_ref()).await
        .map(Arc::new)
        .map_err(|error| Error::IOFailed(error.kind()))?;
    Ok((path.as_ref().to_path_buf(), content))
}

fn default_load_file() -> PathBuf {
    PathBuf::from(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR")))
}

async fn pick_file() -> Result<(PathBuf, Arc<String>), Error> {
    let file_path = rfd::AsyncFileDialog::new().set_title("Choose a file").pick_file().await
        .ok_or(Error::DialogClosed)
        .map(|fileHandle| { fileHandle.path().to_owned() })?;
    load_file(file_path).await
}

async fn save_file(path: Option<PathBuf>, text: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new().set_title("save a file.")
            .save_file().await
            .ok_or(Error::DialogClosed)
            .map(|fileHandle| { fileHandle.path().to_owned() })?
    };
    tokio::fs::write(&path, text).await
        .map_err(|e| { Error::IOFailed(e.kind()) })?;
    Ok(path)
}