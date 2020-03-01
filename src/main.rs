#![cfg_attr(
    not(feature = "gtk_3_10"),
    allow(unused_variables, unused_mut, dead_code)
)]

#[macro_use]
extern crate lazy_static;

extern crate gio;
extern crate glib;
extern crate gtk;
extern crate rand;

lazy_static! {
    static ref COLS: u16 = 9;
    static ref ROWS: u16 = 9;
    static ref MINES: u16 = 10;
}

mod lib;

use gio::prelude::*;
use gtk::{
    prelude::*,
    {ApplicationWindow, Builder, Button},
};
use lib::{
    block::Block, gen_mines, message::Message, position::Position, ui::build_ui, widget::Widget,
};
use std::{cell::RefCell, collections::HashMap, env::args, rc::Rc};

struct Game {
    mines: Vec<Position>,
}

struct Application {
    pub app: Rc<Widget>,
    game: Rc<RefCell<Game>>,
}

impl Application {
    pub fn new(app: &gtk::Application) -> Self {
        let (tx, rx): (glib::Sender<Message>, glib::Receiver<Message>) =
            glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let app = Application {
            app: Rc::new(build_ui(app)),
            game: Rc::new(RefCell::new(Game { mines: Vec::new() })),
        };

        app.update_main_ui_thread(rx);
        app.setup_labels_and_reset(tx.clone());

        app.reset();

        app.bind_click_events(tx.clone())
    }

    fn reset(&self) {
        let game = self.game.clone();

        game.borrow_mut().mines = gen_mines();
    }

    fn update_main_ui_thread(&self, rx: glib::Receiver<Message>) {
        let app = self.app.clone();
        rx.attach(None, move |msg| {
            match msg {
                Message::Reset => {
                    app.reset.set_label("🙂");
                    app.time.set_label("000");
                    app.mines_left.set_label(&format!("{}", *MINES));
                }
                Message::UpdateButton(position, block) => {
                    app.time.set_label(&format!("{:?}", position));

                    let button = block.0;

                    button.set_relief(gtk::ReliefStyle::None);
                    button.set_label("");
                    button.set_can_focus(false);
                }
                Message::SetTime(time) => app.time.set_label(""),
                Message::SetMines(mines) => app.mines_left.set_label(&mines.to_string()),
                _ => {}
            }
            glib::Continue(true)
        });
    }

    fn bind_click_events(self, tx: glib::Sender<Message>) -> Self {
        let app = self.app.clone();
        let mine_event = tx.clone();
        let mines = app.mines.clone();

        mines.iter().for_each(|(position, block)| {
            let event = tx.clone();

            let msg = Message::UpdateButton(position.clone(), block.clone());

            block.0.connect_clicked(move |_| {
                event.send(msg.clone()).expect("couldn't send");
            });
        });

        self
    }

    fn setup_labels_and_reset(&self, tx: glib::Sender<Message>) {
        let app = self.app.clone();

        app.time.set_label("000");
        app.mines_left.set_label(&format!("{}", *MINES));
        app.reset.set_label("🙂");

        app.reset
            .connect_clicked(move |_| tx.send(Message::Reset).expect("reset error"));
    }
}

fn main() {
    let application =
        gtk::Application::new(Some("com.github.Iteam13337.msroj"), Default::default())
            .expect("Initialization failed...");

    application.connect_startup(|app| {
        let application = Application::new(app);
        let application_container = RefCell::new(Some(application));
        app.connect_shutdown(move |_| {
            let application = application_container
                .borrow_mut()
                .take()
                .expect("Shutdown called multiple times");
            drop(application);
        });
    });

    application.connect_activate(|_| {});

    application.run(&args().collect::<Vec<_>>());
}
