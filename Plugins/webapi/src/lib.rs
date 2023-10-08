#[path = "../../../src/scr/sharedtypes.rs"]
mod sharedtypes;

static PLUGIN_NAME:&str = "WebAPI";
static PLUGIN_DESCRIPTION:&str = "Adds support for WebUI & WebAPI..";

#[no_mangle]
pub fn return_info() -> sharedtypes::PluginInfo {
    let callbackvec = vec![sharedtypes::PluginCallback::OnStart];
    sharedtypes::PluginInfo {
        name: PLUGIN_NAME.to_string(),
        description: PLUGIN_DESCRIPTION.to_string(),
        version: 1.00,
        api_version: 1.00,
        callbacks: callbackvec,
        communication: Some(sharedtypes::PluginSharedData { thread: sharedtypes::PluginThreadType::Daemon, com_channel: Some(sharedtypes::PluginCommunicationChannel::pipe("beans".to_string())) }),
    }
}

#[no_mangle]
pub fn on_start() {
    println!("Starting QR Generator");

        QRGenerator::run(Settings::default());

    
}

use iced::widget::qr_code::{self, QRCode};
use iced::widget::{column, container, text, text_input};
use iced::{Alignment, Color, Element, Length, Sandbox, Settings};
use std::thread;
use std::time::Duration;

#[derive(Default)]
struct QRGenerator {
    data: String,
    qr_code: Option<qr_code::State>,
}

#[derive(Debug, Clone)]
enum Message {
    DataChanged(String),
}

impl Sandbox for QRGenerator {
    type Message = Message;

    fn new() -> Self {
        QRGenerator::default()
    }

    fn title(&self) -> String {
        String::from("QR Code Generator - Iced")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::DataChanged(mut data) => {
                data.truncate(100);

                self.qr_code = if data.is_empty() {
                    None
                } else {
                    qr_code::State::new(&data).ok()
                };

                self.data = data;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let title = text("QR Code Generator")
            .size(70)
            .style(Color::from([0.5, 0.5, 0.5]));

        let input =
            text_input("Type the data of your QR code here...", &self.data)
                .on_input(Message::DataChanged)
                .size(30)
                .padding(15);

        let mut content = column![title, input]
            .width(700)
            .spacing(20)
            .align_items(Alignment::Center);

        if let Some(qr_code) = self.qr_code.as_ref() {
            content = content.push(QRCode::new(qr_code).cell_size(10));
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .center_x()
            .center_y()
            .into()
    }
}