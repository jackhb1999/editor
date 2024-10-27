use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use iced::{executor, Application, Command, Element, Length, Sandbox, Settings, Theme};

use iced::widget::{container, row, text, column, horizontal_space, button};

use iced::widget::text_editor;


fn main() -> iced::Result {
    Editor::run(Settings::default())
}

struct Editor {
    content: text_editor::Content,
    error: Option<Error>,
    path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    Open,
    New,
    Save,
    FileSaved(Result<PathBuf, Error>),

}

impl Application for Editor {
    type Executor = executor::Default;
    type Message = Message;

    type Theme = Theme;

    type Flags = ();


    fn new(flages: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                error: None,
                path: None,
            }, Command::perform(load_file(default_load_file()), Message::FileOpened)
        )
    }

    fn title(&self) -> String {
        String::from("This is a text editor.")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
                Command::none()
            }
            Message::FileOpened(Ok((path, content))) => {
                self.content = text_editor::Content::with_text(&content);
                self.path = Some(path);
                Command::none()
            }
            Message::FileOpened(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
            Message::Open => {
                Command::perform(pick_file(), Message::FileOpened)
            }
            Message::New => {
                self.content = text_editor::Content::new();
                self.path = None;
                Command::none()
            }
            Message::Save => {
                let content = self.content.text();
                Command::perform(save_file(self.path.clone(), content), Message::FileSaved)
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                Command::none()
            }
            Message::FileSaved(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let controls = row!(
            button("Open").on_press(Message::Open),
            button("New").on_press(Message::New),
            button("Save").on_press(Message::Save)
        ).spacing(10);
        let input_content = text_editor(&self.content)
            .on_action(Message::Edit)
            .height(Length::Fill);
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
        let status_bar = row!(file_path,horizontal_space(),position);
        container(column!(controls,input_content,status_bar)).padding(5).into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
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