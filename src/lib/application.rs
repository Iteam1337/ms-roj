use crate::{
    lib::{
        game::Game, get_tiles_around, message::Message, position::Position, ui::build_ui,
        widget::Widget,
    },
    MINES,
};
use gtk::{prelude::*, Button};
use std::{cell::RefCell, cmp::max, rc::Rc};

pub struct Application {
    pub widget: Rc<Widget>,
    game: Rc<RefCell<Game>>,
}

impl Application {
    pub fn new(app: &gtk::Application) -> Self {
        let (tx, rx): (glib::Sender<Message>, glib::Receiver<Message>) =
            glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let app = Application {
            widget: Rc::new(build_ui(app)),
            game: Rc::new(RefCell::new(Game::new())),
        };

        app.update_main_ui_thread(rx);
        app.setup_labels_and_reset(tx.clone());

        app.bind_clock(tx.clone());

        app.bind_click_events(tx.clone())
    }

    fn bind_clock(&self, tx: glib::Sender<Message>) {
        let game = self.game.clone();
        let tx = tx.clone();

        let tick = move || {
            let game = game.borrow();

            if game.active {
                let elapsed = game.time.elapsed();

                let secs = elapsed.as_secs();

                let time = format!(
                    "{minutes}:{seconds:0>2}",
                    minutes = (secs / 60) % 60,
                    seconds = secs % 60,
                )
                .to_string();

                tx.send(Message::SetTime(time)).expect("could not set time");
            }

            glib::Continue(true)
        };

        gtk::timeout_add(200, tick);
    }

    fn update_main_ui_thread(&self, rx: glib::Receiver<Message>) {
        let flood_widget = self.widget.clone();
        let flood_game = self.game.clone();
        let flood = move |position: &Position| {
            let mut positions: Vec<Position> = Vec::new();

            {
                let game = flood_game.borrow();
                if let Some(field) = game.field.get(&position) {
                    let field_ref = game.field.to_owned();
                    let positions = get_tiles_around(&position, &field, &field_ref)
                        .iter()
                        .for_each(|(position, _)| positions.push(position.to_owned()));
                }
            }

            if positions.is_empty() {
                return;
            }

            let mut game = flood_game.borrow_mut();
            let mut flagged_count = 0;
            for position in positions {
                let (mut label, mut class_names) = (" ".to_string(), vec![]);
                if let Some(field) = game.field.get_mut(&position) {
                    field.is_clicked = true;

                    if field.is_flagged {
                        field.is_flagged = false;
                        flagged_count += 1;
                    }

                    if field.mines_around != 0 {
                        label = field.mines_around.to_string();
                        class_names.push("btn_close".to_string());
                        class_names.push(field.mines_around_class_name());
                    } else {
                        class_names.push("btn_empty".to_string());
                    }
                }

                let id = format!("mine_{}{}", position.0, position.1);
                let button: Option<Button> = flood_widget.builder.get_object(&id);
                if let Some(button) = button {
                    button.set_relief(gtk::ReliefStyle::None);
                    button.set_label(&label);
                    button.set_can_focus(false);
                    let ctx = button.get_style_context();

                    for class_name in ctx.list_classes() {
                        if class_name.starts_with("btn_") {
                            ctx.remove_class(&class_name);
                        }
                    }

                    for class in class_names {
                        ctx.add_class(&class);
                    }
                }
            }

            if flagged_count > 0 {
                let mut mines: i16 = flood_widget
                    .mines_left
                    .get_label()
                    .unwrap()
                    .parse()
                    .unwrap_or(0);

                mines = max(mines + flagged_count, 0);

                flood_widget.mines_left.set_label(&mines.to_string());
            }
        };

        let show_all_mines_widget = self.widget.clone();
        let show_all_mines_game = self.game.clone();
        let show_all_mines = move || {
            let game = show_all_mines_game.borrow();

            for mine in &game.mines {
                let id = format!("mine_{}{}", mine.0, mine.1);
                let button: Option<Button> = show_all_mines_widget.builder.get_object(&id);

                if let Some(button) = button {
                    if let Some(label) = button.get_label() {
                        if label == "🔥" {
                            continue;
                        }
                    }

                    button.set_relief(gtk::ReliefStyle::None);
                    button.set_label(&"💣");
                    button.set_can_focus(false);
                    let ctx = button.get_style_context();
                    ctx.add_class(&"btn_mine");
                }
            }

            show_all_mines_widget.reset.set_label(&"👻");
        };

        let check_if_completed_widget = self.widget.clone();
        let check_if_completed_game = self.game.clone();
        let check_if_completed = move || {
            let check_if_completed_game = check_if_completed_game.clone();
            let widget = check_if_completed_widget.clone();
            let completed: bool;

            {
                let game = check_if_completed_game.borrow();
                let mines = game
                    .field
                    .iter()
                    .filter_map(|(position, field)| {
                        if game.mines.contains(&position) && !field.is_clicked && field.is_flagged {
                            Some(position)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&Position>>();
                let left = game
                    .field
                    .iter()
                    .filter_map(|(position, field)| {
                        if !field.is_clicked {
                            Some(position)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&Position>>();
                completed = mines.len() == *MINES as usize && mines.len() == left.len();
            }

            if completed {
                let mut game = check_if_completed_game.borrow_mut();
                game.ended = true;
                game.active = false;
                widget.reset.set_label("🎊");
                widget.mines_left.set_label(&"0");
                widget.mines.iter().for_each(|(position, block)| {
                    block.0.set_can_focus(false);
                });
            }
        };

        let widget = self.widget.clone();
        let game = self.game.clone();
        rx.attach(None, move |msg| {
            match msg {
                Message::Reset => {
                    widget.reset.set_label("🙂");
                    widget.time.set_label("0:00");
                    widget.mines_left.set_label(&format!("{}", *MINES));
                    game.borrow_mut().new_mines();

                    widget.mines.iter().for_each(|(position, block)| {
                        block.0.set_label(" ");
                        block.0.set_can_focus(true);
                        block.0.set_relief(gtk::ReliefStyle::Normal);
                        let ctx = block.0.get_style_context();

                        for class_name in ctx.list_classes() {
                            if class_name.starts_with("btn_") {
                                ctx.remove_class(&class_name);
                            }
                        }
                    });
                }
                Message::End => {
                    let mut game = game.borrow_mut();
                    game.ended = true;
                    game.active = false;
                    widget.reset.set_label("🎊");
                    widget.mines_left.set_label(&"0");
                    widget.mines.iter().for_each(|(position, block)| {
                        block.0.set_can_focus(false);
                    });
                }
                Message::UpdateButton(position, block, flag) => {
                    let button = block.0;
                    let mut empty = false;
                    let mut game_ended = false;

                    'mut_closure: {
                        let mut game = game.borrow_mut();

                        if game.ended {
                            break 'mut_closure;
                        }

                        button.set_relief(gtk::ReliefStyle::None);

                        if !game.active {
                            game.start_timer();
                        }

                        let field = game.field.get_mut(&position);

                        if field.is_none() {
                            break 'mut_closure;
                        }

                        let mut field = field.unwrap();

                        if flag && field.is_clicked == false {
                            let flagged = !field.is_flagged;
                            let (label, class, inc_mine_count) = if flagged {
                                ("🚩", "btn_flag", false)
                            } else {
                                (" ", "", true)
                            };

                            if !inc_mine_count {
                                let mines_left =
                                    widget.mines_left.get_label().unwrap().parse().unwrap_or(0);
                                if mines_left <= 0 {
                                    break 'mut_closure;
                                }
                            }

                            field.is_flagged = flagged;
                            button.set_relief(gtk::ReliefStyle::Normal);
                            button.set_label(&label.to_string());

                            let ctx = button.get_style_context();

                            for class_name in ctx.list_classes() {
                                if class_name.starts_with("btn_") {
                                    ctx.remove_class(&class_name);
                                }
                            }

                            ctx.add_class(&class);

                            let mut mines: i16 =
                                widget.mines_left.get_label().unwrap().parse().unwrap_or(0);

                            mines = max(if inc_mine_count { mines + 1 } else { mines - 1 }, 0);

                            widget.mines_left.set_label(&mines.to_string());

                            break 'mut_closure;
                        }

                        field.is_clicked = true;

                        let (label, class_names) = if field.is_mine {
                            (
                                "🔥".to_string(),
                                vec!["btn_mine", "btn_mine_clicked"]
                                    .iter()
                                    .map(|str| str.to_string())
                                    .collect(),
                            )
                        } else if field.mines_around == 0 {
                            (" ".to_string(), vec!["btn_empty".to_string()])
                        } else {
                            (
                                field.mines_around.to_string(),
                                vec!["btn_close".to_string(), field.mines_around_class_name()],
                            )
                        };

                        if field.is_mine {
                            game.active = false;
                            game.ended = true;
                        }

                        button.set_label(&label);
                        button.set_can_focus(false);

                        let ctx = button.get_style_context();

                        for class_name in ctx.list_classes() {
                            if class_name.starts_with("btn_") {
                                ctx.remove_class(&class_name);
                            }
                        }

                        for class in &class_names {
                            ctx.add_class(class);
                        }

                        if game.ended {
                            game_ended = true;
                        }

                        if class_names.contains(&"btn_empty".to_string()) {
                            empty = true;
                        }
                    }

                    if empty {
                        flood(&position);
                    } else if game_ended {
                        show_all_mines();
                    } else {
                        check_if_completed();
                    }
                }
                Message::SetTime(time) => widget.time.set_label(&time),
                Message::SetMines(mines) => widget.mines_left.set_label(&mines),
                _ => {}
            }
            glib::Continue(true)
        });
    }

    fn bind_click_events(self, tx: glib::Sender<Message>) -> Self {
        let widget = self.widget.clone();
        let mine_event = tx.clone();
        let mines = widget.mines.clone();

        mines.iter().for_each(|(position, block)| {
            let send = tx.clone();
            let msg = Message::UpdateButton(position.clone(), block.clone(), true);
            block.0.connect_button_release_event(move |_, event| {
                match event.get_button() {
                    3 => send.send(msg.clone()).expect("couldn't send"),
                    _ => {}
                };
                Inhibit(false)
            });

            let send = tx.clone();
            let msg = Message::UpdateButton(position.clone(), block.clone(), true);
            block.0.connect_key_press_event(move |_, key| {
                match key.get_keyval() {
                    102 => send.send(msg.clone()).expect("couldn't send"),
                    _ => {}
                }

                Inhibit(false)
            });

            let send = tx.clone();
            let msg = Message::UpdateButton(position.clone(), block.clone(), false);
            block.0.connect_clicked(move |_| {
                send.send(msg.clone()).expect("couldn't send");
            });
        });

        self
    }

    fn setup_labels_and_reset(&self, tx: glib::Sender<Message>) {
        let widget = self.widget.clone();

        widget.time.set_label("0:00");
        widget.mines_left.set_label(&format!("{}", *MINES));
        widget.reset.set_label("🙂");

        widget
            .reset
            .connect_clicked(move |_| tx.send(Message::Reset).expect("reset error"));
    }
}
