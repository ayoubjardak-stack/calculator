//! A calculator built with iced. The same code runs on Windows (see `main.rs`)
//! and on Android (see `android_main` below).

use iced::keyboard::{self, key::Named, Key};
use iced::theme::Palette;
use iced::widget::{button, column, container, responsive, row, scrollable, space, text, toggler, Row};
use iced::{
    window, Alignment, Background, Border, Color, Element, Font, Length, Shadow, Size,
    Subscription, Task, Theme,
};

const BASIC_SIZE: Size = Size::new(360.0, 580.0);
const SCIENTIFIC_SIZE: Size = Size::new(860.0, 580.0);

/// Starts the app.
pub fn run() -> iced::Result {
    #[cfg_attr(not(target_os = "android"), allow(unused_mut))]
    let mut app = iced::application(App::default, App::update, App::view)
        .title("Calculator")
        .theme(|app: &App| app.current_theme())
        .subscription(App::subscription)
        .window(window::Settings {
            size: BASIC_SIZE,
            icon: window::icon::from_rgba(include_bytes!("../assets/icon_128.rgba").to_vec(), 128, 128).ok(),
            ..window::Settings::default()
        });

    #[cfg(target_os = "android")]
    {
        for font in android::system_fonts() {
            app = app.font(font);
        }
        app = app.default_font(Font::with_name(android::UI_FONT));
    }

    app.run()
}

/// Entry point called by Android's NativeActivity.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Send logs and crash messages to logcat (`adb logcat -s calculator`).
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("calculator"),
    );
    std::panic::set_hook(Box::new(|info| log::error!("crashed: {info}")));

    let _ = iced_winit::ANDROID_APP.set(app);
    if let Err(e) = run() {
        log::error!("calculator failed: {e}");
    }
}

/// Space around the whole app.
///
/// On Android: 0.4 cm on every side (iced's logical pixels are Android dp,
/// 1/160 inch, so 0.4 cm ≈ 25 dp), plus the size of the status bar and
/// navigation bar if they are drawn over the app.
fn screen_padding() -> iced::Padding {
    #[cfg(target_os = "android")]
    {
        let bars = android::system_bars();
        let base = 25.0;
        iced::Padding {
            top: base + bars.top,
            right: base + bars.right,
            bottom: base + bars.bottom,
            left: base + bars.left,
        }
    }
    #[cfg(not(target_os = "android"))]
    iced::Padding::new(16.0)
}

#[cfg(target_os = "android")]
mod android {
    /// How much of the window the system bars cover, in dp.
    /// Zero when Android already keeps the app clear of them.
    pub fn system_bars() -> iced::Padding {
        let Some(app) = iced_winit::ANDROID_APP.get() else { return iced::Padding::ZERO };
        let Some(window) = app.native_window() else { return iced::Padding::ZERO };
        let rect = app.content_rect();
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return iced::Padding::ZERO;
        }
        let dp = app.config().density().unwrap_or(160) as f32 / 160.0;
        let px = |v: i32| (v.max(0) as f32) / dp;
        iced::Padding {
            top: px(rect.top),
            left: px(rect.left),
            right: px(window.width() - rect.right),
            bottom: px(window.height() - rect.bottom),
        }
    }

    /// fontdb doesn't scan Android's font folder, so we load the fonts we need ourselves.
    pub const UI_FONT: &str = "Roboto";
    pub const MONO_FONT: &str = "Droid Sans Mono";

    pub fn system_fonts() -> Vec<Vec<u8>> {
        let Ok(dir) = std::fs::read_dir("/system/fonts") else { return Vec::new() };
        let mut fonts = Vec::new();
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let wanted = name.starts_with("Roboto-Regular")
                || name.starts_with("Roboto-Light")
                || name == "Roboto[ital,wdth,wght].ttf"
                || name.starts_with("DroidSansMono")
                || name.starts_with("NotoSansSymbols")
                || name.starts_with("NotoSansMath");
            if wanted {
                if let Ok(bytes) = std::fs::read(entry.path()) {
                    fonts.push(bytes);
                }
            }
        }
        fonts
    }
}

// ───────────────────────────── App ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Calculator,
    Settings,
}

#[derive(Debug, Clone)]
enum Message {
    Input(Input),
    OpenSettings,
    CloseSettings,
    ThemeSelected(usize),
    ToggleRounded(bool),
    ToggleScientific(bool),
    ToggleSecond,
    SizeSelected(UiSize),
    Keyboard(keyboard::Event),
}

/// How big the buttons and text are (Settings → Size).
#[derive(Debug, Clone, Copy, PartialEq)]
enum UiSize {
    Compact,
    Normal,
    Large,
}

impl UiSize {
    const ALL: [UiSize; 3] = [UiSize::Compact, UiSize::Normal, UiSize::Large];

    fn scale(self) -> f32 {
        match self {
            UiSize::Compact => 0.8,
            UiSize::Normal => 1.0,
            UiSize::Large => 1.25,
        }
    }

    fn name(self) -> &'static str {
        match self {
            UiSize::Compact => "Compact",
            UiSize::Normal => "Normal",
            UiSize::Large => "Large",
        }
    }
}

/// A theme the user can pick. `theme: None` means "follow the system".
struct ThemeOption {
    name: &'static str,
    theme: Option<Theme>,
}

struct App {
    engine: Engine,
    screen: Screen,
    themes: Vec<ThemeOption>,
    selected: usize,
    rounded: bool,
    scientific: bool,
    /// The "2nd" key: swaps some scientific keys for their inverse.
    second: bool,
    size: UiSize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            engine: Engine::default(),
            screen: Screen::Calculator,
            themes: theme_options(),
            selected: 0,
            rounded: true,
            scientific: false,
            second: false,
            size: UiSize::Normal,
        }
    }
}

impl App {
    /// `None` lets iced pick Light or Dark based on the OS setting.
    fn current_theme(&self) -> Option<Theme> {
        self.themes[self.selected].theme.clone()
    }

    fn font(&self) -> Font {
        if self.themes[self.selected].name == "Hacker" {
            #[cfg(target_os = "android")]
            return Font::with_name(android::MONO_FONT);
            #[cfg(not(target_os = "android"))]
            return Font::MONOSPACE;
        }
        #[cfg(target_os = "android")]
        return Font::with_name(android::UI_FONT);
        #[cfg(not(target_os = "android"))]
        Font::DEFAULT
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Input(input) => self.engine.press(input),
            Message::OpenSettings => self.screen = Screen::Settings,
            Message::CloseSettings => self.screen = Screen::Calculator,
            Message::ThemeSelected(i) => self.selected = i,
            Message::ToggleRounded(on) => self.rounded = on,
            Message::ToggleSecond => self.second = !self.second,
            Message::SizeSelected(size) => self.size = size,
            Message::ToggleScientific(on) => {
                self.scientific = on;
                let size = if on { SCIENTIFIC_SIZE } else { BASIC_SIZE };
                return window::latest().and_then(move |id| window::resize(id, size));
            }
            Message::Keyboard(event) => {
                let keyboard::Event::KeyPressed { modified_key, .. } = event else {
                    return Task::none();
                };
                if self.screen == Screen::Settings {
                    // Escape on a PC, the back button on Android.
                    if matches!(modified_key, Key::Named(Named::Escape | Named::BrowserBack | Named::GoBack)) {
                        self.screen = Screen::Calculator;
                    }
                    return Task::none();
                }
                if let Some(input) = input_from_key(&modified_key) {
                    self.engine.press(input);
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().map(Message::Keyboard)
    }

    fn radius(&self) -> f32 {
        if self.rounded { 18.0 * self.size.scale() } else { 4.0 }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.screen {
            Screen::Calculator => self.calculator_view(),
            Screen::Settings => self.settings_view(),
        };
        container(content)
            .padding(screen_padding())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn calculator_view(&self) -> Element<'_, Message> {
        responsive(move |size| self.calculator_layout(size)).into()
    }

    fn calculator_layout(&self, size: Size) -> Element<'_, Message> {
        let r = self.radius();
        let font = self.font();
        let sci = self.scientific;
        let sc = self.size.scale();

        let header = row![
            text("Calculator").size(20.0 * sc),
            space::horizontal(),
            button(text("fx").size(18.0 * sc).font(font))
                .on_press(Message::ToggleScientific(!sci))
                .padding([4.0 * sc, 12.0 * sc])
                .style(move |t, s| {
                    key_style(t, s, if sci { Kind::Active } else { Kind::Function }, r)
                }),
            button(text("⚙").size(20.0 * sc))
                .on_press(Message::OpenSettings)
                .padding([4.0 * sc, 12.0 * sc])
                .style(move |t, s| key_style(t, s, Kind::Function, r)),
        ]
        .spacing(8.0 * sc)
        .align_y(Alignment::Center);

        let display = self.engine.display.clone();
        let expression = self.engine.expression();

        let mut flags = Vec::new();
        if sci && self.engine.radians {
            flags.push("Rad");
        }
        if self.engine.memory != 0.0 {
            flags.push("M");
        }
        let dim = |t: &Theme| text::Style {
            color: Some(t.extended_palette().background.strong.color),
        };

        let flags = flags.join("  ");
        let pad_x = 18.0 * sc;

        // The text shrinks to fit the width, so long numbers never run off the screen.
        let readout = responsive(move |size| {
            let width = (size.width - 2.0 * pad_x).max(40.0);
            let fit = |chars: usize, wanted: f32| wanted.min(width / (chars.max(1) as f32 * 0.6));
            let number = fit(display.chars().count(), 60.0 * sc).max(16.0);
            let line = fit(expression.chars().count(), 18.0 * sc).max(10.0);

            column![
                row![
                    text(flags.clone()).size(14.0 * sc).font(font).style(dim),
                    space::horizontal(),
                    text(expression.clone())
                        .size(line)
                        .font(font)
                        .wrapping(text::Wrapping::None)
                        .style(dim),
                ]
                .align_y(Alignment::Center),
                space::vertical(),
                text(display.clone()).size(number).font(font).wrapping(text::Wrapping::None),
            ]
            .align_x(Alignment::End)
            .spacing(6.0 * sc)
            .into()
        });

        let screen = container(readout)
        .padding([16.0 * sc, pad_x])
        .width(Length::Fill)
        // The keys keep their own size; the display takes whatever is left.
        .height(Length::Fill)
        .align_x(Alignment::End)
        .style(move |t: &Theme| {
            let p = t.extended_palette();
            container::Style {
                background: Some(p.background.weakest.color.into()),
                text_color: Some(p.background.weakest.text),
                border: Border { radius: r.into(), ..Border::default() },
                ..container::Style::default()
            }
        });

        // On a narrow screen (a phone), scientific keys go above the basic ones
        // instead of beside them.
        let spacing = 14.0 * sc;
        let header_height = 40.0 * sc;
        let min_display = 110.0 * sc;
        let room_for_keys = size.height - header_height - min_display - 2.0 * spacing;
        let pad = self.keypad(size.width < 600.0, size.width, room_for_keys);

        column![header, screen, pad].spacing(spacing).into()
    }

    /// The buttons. `stacked` puts the scientific keys above the basic ones.
    fn keypad(&self, stacked: bool, width: f32, max_height: f32) -> Element<'_, Message> {
        let r = self.radius();
        let font = self.font();
        let sc = self.size.scale();
        let gap = 10.0 * sc;
        let stacked = stacked && self.scientific;

        // Keys get a natural height from their width (a bit wider than tall)
        // instead of stretching to fill a tall phone screen.
        let columns = if self.scientific && !stacked { 10.0 } else { 4.0 };
        let key_width = (width - gap * (columns - 1.0)) / columns;
        let mut key_height = (key_width * 0.8).clamp(36.0, 80.0 * sc);
        let sci_gap = gap * 0.8;
        let sci_key_width = if stacked { (width - sci_gap * 5.0) / 6.0 } else { key_width };
        let mut sci_height = if stacked { (sci_key_width * 0.72).clamp(30.0, 52.0 * sc) } else { key_height };

        // On short screens, shrink the keys so the display still fits.
        let gaps = 4.0 * gap + if stacked { 5.0 * sci_gap } else { 0.0 };
        let wanted = 5.0 * key_height + if stacked { 5.0 * sci_height } else { 0.0 };
        if wanted + gaps > max_height && wanted > 0.0 {
            let k = ((max_height - gaps) / wanted).max(0.4);
            key_height *= k;
            sci_height *= k;
        }

        // Text follows the Size setting but always fits inside its key.
        let basic_text = (26.0 * sc).min(key_width * 0.42).min(key_height * 0.5);
        let sci_text = (17.0 * sc).min(sci_key_width * 0.26).min(sci_height * 0.42).max(9.0);

        // Basic keypad: 5 rows × 4 columns.
        let k = |label: &str, input: Input, kind: Kind| {
            key(label.to_string(), Message::Input(input), kind, r, font, basic_text)
        };
        let d = |c: char| k(&c.to_string(), Input::Digit(c), Kind::Digit);
        let pending = self.engine.pending_operator();
        let op = |b: Bin| if pending == Some(b) { Kind::OperatorActive } else { Kind::Operator };
        let basic: [[Element<'_, Message>; 4]; 5] = [
            [
                k("C", Input::Clear, Kind::Clear),
                k("±", Input::Negate, Kind::Function),
                k("%", Input::Percent, Kind::Function),
                k("÷", Input::Binary(Bin::Div), op(Bin::Div)),
            ],
            [d('7'), d('8'), d('9'), k("×", Input::Binary(Bin::Mul), op(Bin::Mul))],
            [d('4'), d('5'), d('6'), k("−", Input::Binary(Bin::Sub), op(Bin::Sub))],
            [d('1'), d('2'), d('3'), k("+", Input::Binary(Bin::Add), op(Bin::Add))],
            [
                k("⌫", Input::Backspace, Kind::Function),
                d('0'),
                k(".", Input::Dot, Kind::Digit),
                k("=", Input::Equals, Kind::Equals),
            ],
        ];

        let mut pad = column![].spacing(gap);
        let mut sci_rows =
            if self.scientific { Some(self.scientific_keys(r, font, sci_text)) } else { None };
        if stacked {
            let mut sci = column![].spacing(sci_gap);
            for line in sci_rows.take().into_iter().flatten() {
                sci = sci.push(Row::with_children(line).spacing(sci_gap).height(sci_height));
            }
            pad = pad.push(sci);
        }
        for (i, basic_row) in basic.into_iter().enumerate() {
            let mut line: Vec<Element<'_, Message>> = Vec::new();
            if let Some(rows) = sci_rows.as_mut() {
                line.append(&mut rows[i]);
            }
            line.extend(basic_row);
            pad = pad.push(Row::with_children(line).spacing(gap).height(key_height));
        }
        pad.into()
    }

    /// The six extra columns on the left, laid out like the macOS scientific calculator.
    fn scientific_keys(&self, r: f32, font: Font, size: f32) -> Vec<Vec<Element<'_, Message>>> {
        let s = |label: &str, input: Input| {
            key(label.to_string(), Message::Input(input), Kind::Function, r, font, size)
        };
        let u = |label: &str, f: Un| s(label, Input::Unary(f));
        let b = |label: &str, op: Bin| s(label, Input::Binary(op));
        let nd = self.second;

        let second_key = key(
            "2nd".into(),
            Message::ToggleSecond,
            if nd { Kind::Active } else { Kind::Function },
            r,
            font,
            size,
        );
        let rad_key = key(
            "Rad".into(),
            Message::Input(Input::ToggleAngle),
            if self.engine.radians { Kind::Active } else { Kind::Function },
            r,
            font,
            size,
        );

        vec![
            vec![
                s("(", Input::LParen),
                s(")", Input::RParen),
                s("mc", Input::MemClear),
                s("m+", Input::MemAdd),
                s("m−", Input::MemSub),
                s("mr", Input::MemRecall),
            ],
            vec![
                second_key,
                u("x²", Un::Square),
                u("x³", Un::Cube),
                b("xʸ", Bin::Pow),
                if nd { b("yˣ", Bin::RevPow) } else { u("eˣ", Un::Exp) },
                if nd { u("2ˣ", Un::Pow2) } else { u("10ˣ", Un::Pow10) },
            ],
            vec![
                u("1/x", Un::Recip),
                u("√x", Un::Sqrt),
                u("∛x", Un::Cbrt),
                b("ʸ√x", Bin::Root),
                if nd { b("logy", Bin::LogBase) } else { u("ln", Un::Ln) },
                if nd { u("log₂", Un::Log2) } else { u("log₁₀", Un::Log10) },
            ],
            vec![
                u("x!", Un::Fact),
                if nd { u("sin⁻¹", Un::Asin) } else { u("sin", Un::Sin) },
                if nd { u("cos⁻¹", Un::Acos) } else { u("cos", Un::Cos) },
                if nd { u("tan⁻¹", Un::Atan) } else { u("tan", Un::Tan) },
                s("e", Input::Const(Const::E)),
                b("EE", Bin::EE),
            ],
            vec![
                rad_key,
                if nd { u("sinh⁻¹", Un::Asinh) } else { u("sinh", Un::Sinh) },
                if nd { u("cosh⁻¹", Un::Acosh) } else { u("cosh", Un::Cosh) },
                if nd { u("tanh⁻¹", Un::Atanh) } else { u("tanh", Un::Tanh) },
                s("π", Input::Const(Const::Pi)),
                s("Rand", Input::Const(Const::Rand)),
            ],
        ]
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let r = self.radius();

        let header = row![
            button(text("← Back").size(16))
                .on_press(Message::CloseSettings)
                .padding([6, 14])
                .style(move |t, s| key_style(t, s, Kind::Function, r)),
            space::horizontal(),
            text("Settings").size(22),
        ]
        .align_y(Alignment::Center);

        let setting = |label: &'static str, on: bool, msg: fn(bool) -> Message| {
            row![
                text(label).size(16),
                space::horizontal(),
                toggler(on).on_toggle(msg),
            ]
            .align_y(Alignment::Center)
        };

        // Compact / Normal / Large, as one row of buttons.
        let current = self.size;
        let size_picker = Row::with_children(UiSize::ALL.map(|size| {
            let kind = if size == current { Kind::Active } else { Kind::Function };
            button(text(size.name()).size(15).width(Length::Fill).align_x(Alignment::Center))
                .on_press(Message::SizeSelected(size))
                .padding([10, 6])
                .width(Length::Fill)
                .style(move |t, s| key_style(t, s, kind, r))
                .into()
        }))
        .spacing(8);

        let mut grid = column![].spacing(10);
        for (n, pair) in self.themes.chunks(2).enumerate() {
            let mut line = row![].spacing(10);
            for (j, option) in pair.iter().enumerate() {
                let i = n * 2 + j;
                line = line.push(theme_card(option, i, i == self.selected, r));
            }
            if pair.len() == 1 {
                line = line.push(space::horizontal());
            }
            grid = grid.push(line);
        }

        // Each group of settings sits on its own rounded card.
        let card = move |content: Element<'static, Message>| -> Element<'static, Message> {
            container(content)
                .padding(14)
                .width(Length::Fill)
                .style(move |t: &Theme| {
                    let p = t.extended_palette();
                    container::Style {
                        background: Some(p.background.weakest.color.into()),
                        border: Border { radius: r.into(), ..Border::default() },
                        ..container::Style::default()
                    }
                })
                .into()
        };

        let options = column![
            section("Mode"),
            card(setting("Scientific calculator", self.scientific, Message::ToggleScientific).into()),
            section("Size"),
            card(size_picker.into()),
            section("Appearance"),
            card(setting("Rounded buttons", self.rounded, Message::ToggleRounded).into()),
            section("Theme"),
            grid,
        ]
        .spacing(10);

        column![
            header,
            // Everything below the header scrolls, so small phones can reach all of it.
            scrollable(container(options).padding(iced::Padding::ZERO.right(14)))
                .height(Length::Fill),
        ]
        .spacing(14)
        .into()
    }
}

fn section(title: &str) -> Element<'_, Message> {
    text(title)
        .size(14)
        .style(|t: &Theme| text::Style {
            color: Some(Color { a: 0.6, ..t.palette().text }),
        })
        .into()
}

fn theme_card(option: &ThemeOption, index: usize, selected: bool, radius: f32) -> Element<'static, Message> {
    // "System" has no fixed colors, so preview it as a half light / half dark card.
    let system = option.theme.is_none();
    let p = match &option.theme {
        Some(theme) => theme.palette(),
        None => Palette { primary: Color::WHITE, success: Color::BLACK, ..Theme::Dark.palette() },
    };
    let swatch = |c: Color| {
        container(space::horizontal())
            .width(18)
            .height(18)
            .style(move |_: &Theme| container::Style {
                background: Some(c.into()),
                border: Border { radius: 9.0.into(), ..Border::default() },
                ..container::Style::default()
            })
    };

    let content = column![
        row![swatch(p.primary), swatch(p.success), swatch(p.danger), swatch(p.text)].spacing(6),
        text(if system { "System (auto)" } else { option.name }).size(14),
    ]
    .spacing(8);

    button(content)
        .on_press(Message::ThemeSelected(index))
        .padding(12)
        .width(Length::Fill)
        .style(move |current: &Theme, status| {
            let accent = current.extended_palette().primary.base.color;
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let background = if system {
                Background::Gradient(
                    iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                        .add_stop(0.0, Color::from_rgb8(0x20, 0x22, 0x25))
                        .add_stop(0.55, Color::from_rgb8(0x20, 0x22, 0x25))
                        .add_stop(0.55, Color::from_rgb8(0x5A, 0x5C, 0x60))
                        .add_stop(1.0, Color::from_rgb8(0x9A, 0x9C, 0xA0))
                        .into(),
                )
            } else {
                p.background.into()
            };
            button::Style {
                background: Some(background),
                text_color: p.text,
                border: Border {
                    color: if selected || hovered { accent } else { Color { a: 0.15, ..p.text } },
                    width: if selected { 3.0 } else { 1.0 },
                    radius: radius.into(),
                },
                ..button::Style::default()
            }
        })
        .into()
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Digit,
    Function,
    Operator,
    /// The operator just pressed, waiting for the next number (shown inverted).
    OperatorActive,
    Equals,
    Clear,
    /// A toggle key that is switched on (2nd, Rad, fx).
    Active,
}

fn key(
    label: String,
    message: Message,
    kind: Kind,
    radius: f32,
    font: Font,
    size: f32,
) -> Element<'static, Message> {
    button(
        text(label)
            .size(size)
            .font(font)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .on_press(message)
    .style(move |t, s| key_style(t, s, kind, radius))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn key_style(theme: &Theme, status: button::Status, kind: Kind, radius: f32) -> button::Style {
    let p = theme.extended_palette();
    let pair = match kind {
        Kind::Digit => p.background.weak,
        Kind::Function => p.background.strong,
        Kind::Operator => p.primary.base,
        Kind::OperatorActive => iced::theme::palette::Pair {
            color: p.primary.base.text,
            text: p.primary.base.color,
        },
        Kind::Equals => p.success.base,
        Kind::Clear => p.danger.base,
        Kind::Active => p.primary.weak,
    };
    let (bg, lift) = match status {
        button::Status::Hovered => (mix(pair.color, pair.text, 0.12), 4.0),
        button::Status::Pressed => (mix(pair.color, Color::BLACK, 0.15), 0.0),
        button::Status::Disabled => (Color { a: 0.5, ..pair.color }, 0.0),
        button::Status::Active => (pair.color, 2.0),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: pair.text,
        border: Border { radius: radius.into(), ..Border::default() },
        shadow: Shadow {
            color: Color { a: 0.25, ..Color::BLACK },
            offset: iced::Vector::new(0.0, lift / 2.0),
            blur_radius: lift * 2.0,
        },
        ..button::Style::default()
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a,
    }
}

fn theme_options() -> Vec<ThemeOption> {
    let rgb = Color::from_rgb8;
    let make = |name: &'static str, c: [(u8, u8, u8); 6]| {
        let [bg, txt, pri, ok, warn, bad] = c.map(|(r, g, b)| rgb(r, g, b));
        ThemeOption {
            name,
            theme: Some(Theme::custom(
                name,
                Palette { background: bg, text: txt, primary: pri, success: ok, warning: warn, danger: bad },
            )),
        }
    };
    vec![
        ThemeOption { name: "System", theme: None },
        make("White", [(0xFF, 0xFF, 0xFF), (0x11, 0x11, 0x11), (0x1A, 0x1A, 0x1A), (0x00, 0x00, 0x00), (0x88, 0x88, 0x88), (0x6B, 0x6B, 0x6B)]),
        make("Black", [(0x00, 0x00, 0x00), (0xFF, 0xFF, 0xFF), (0xF2, 0xF2, 0xF2), (0xFF, 0xFF, 0xFF), (0xBB, 0xBB, 0xBB), (0x8A, 0x8A, 0x8A)]),
        make("Hacker", [(0x05, 0x0A, 0x05), (0x33, 0xFF, 0x33), (0x00, 0xB3, 0x2D), (0x39, 0xFF, 0x14), (0xC8, 0xFF, 0x00), (0xFF, 0x3B, 0x3B)]),
        make("Ocean", [(0x0F, 0x1E, 0x2E), (0xE6, 0xF1, 0xFF), (0x1F, 0xA2, 0xFF), (0x12, 0xD8, 0xA5), (0xFF, 0xC8, 0x57), (0xFF, 0x5A, 0x5F)]),
        make("Sunset", [(0x2B, 0x1B, 0x2E), (0xFF, 0xEF, 0xE5), (0xFF, 0x7E, 0x5F), (0xFE, 0xB4, 0x7B), (0xFF, 0xD1, 0x66), (0xE6, 0x39, 0x46)]),
        make("Midnight", [(0x10, 0x10, 0x24), (0xE0, 0xE0, 0xFF), (0x7C, 0x5C, 0xFF), (0x3D, 0xDC, 0x97), (0xFF, 0xB4, 0x00), (0xFF, 0x4D, 0x6D)]),
    ]
}

fn input_from_key(key: &Key) -> Option<Input> {
    match key {
        Key::Named(Named::Enter) => Some(Input::Equals),
        Key::Named(Named::Backspace) => Some(Input::Backspace),
        Key::Named(Named::Escape | Named::Delete) => Some(Input::Clear),
        Key::Character(c) => match c.as_str() {
            d @ ("0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") => {
                d.chars().next().map(Input::Digit)
            }
            "." | "," => Some(Input::Dot),
            "+" => Some(Input::Binary(Bin::Add)),
            "-" => Some(Input::Binary(Bin::Sub)),
            "*" | "x" | "X" => Some(Input::Binary(Bin::Mul)),
            "/" => Some(Input::Binary(Bin::Div)),
            "^" => Some(Input::Binary(Bin::Pow)),
            "(" => Some(Input::LParen),
            ")" => Some(Input::RParen),
            "!" => Some(Input::Unary(Un::Fact)),
            "=" => Some(Input::Equals),
            "%" => Some(Input::Percent),
            "c" | "C" => Some(Input::Clear),
            "n" | "N" => Some(Input::Negate),
            _ => None,
        },
        _ => None,
    }
}

// ─────────────────────────── Engine ────────────────────────────

/// Operators that take two numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    /// x ^ y
    Pow,
    /// y ^ x (the "2nd" version of eˣ)
    RevPow,
    /// y-th root of x
    Root,
    /// log base y of x
    LogBase,
    /// x × 10^y
    EE,
}

impl Bin {
    fn precedence(self) -> u8 {
        match self {
            Bin::Add | Bin::Sub => 1,
            Bin::Mul | Bin::Div => 2,
            _ => 3,
        }
    }

    fn right_assoc(self) -> bool {
        self.precedence() == 3
    }

    fn apply(self, a: f64, b: f64) -> Option<f64> {
        let r = match self {
            Bin::Add => a + b,
            Bin::Sub => a - b,
            Bin::Mul => a * b,
            Bin::Div if b == 0.0 => return None,
            Bin::Div => a / b,
            Bin::Pow => a.powf(b),
            Bin::RevPow => b.powf(a),
            Bin::Root if b == 0.0 => return None,
            // Odd roots of negative numbers are real: ³√-8 = -2.
            Bin::Root if a < 0.0 && b.fract() == 0.0 && (b as i64) % 2 != 0 => -(-a).powf(1.0 / b),
            Bin::Root => a.powf(1.0 / b),
            Bin::LogBase => a.ln() / b.ln(),
            Bin::EE => a * 10f64.powf(b),
        };
        r.is_finite().then_some(r)
    }

    fn symbol(self) -> &'static str {
        match self {
            Bin::Add => "+",
            Bin::Sub => "−",
            Bin::Mul => "×",
            Bin::Div => "÷",
            Bin::Pow => "^",
            Bin::RevPow => "yˣ",
            Bin::Root => "ʸ√",
            Bin::LogBase => "logy",
            Bin::EE => "E",
        }
    }
}

/// Functions that act on the number on screen right away.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Un {
    Square,
    Cube,
    Exp,
    Pow10,
    Pow2,
    Recip,
    Sqrt,
    Cbrt,
    Ln,
    Log10,
    Log2,
    Fact,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
}

impl Un {
    fn apply(self, x: f64, radians: bool) -> f64 {
        let to_rad = |v: f64| if radians { v } else { v.to_radians() };
        let from_rad = |v: f64| if radians { v } else { v.to_degrees() };
        match self {
            Un::Square => x * x,
            Un::Cube => x * x * x,
            Un::Exp => x.exp(),
            Un::Pow10 => 10f64.powf(x),
            Un::Pow2 => 2f64.powf(x),
            Un::Recip if x == 0.0 => f64::NAN,
            Un::Recip => 1.0 / x,
            Un::Sqrt => x.sqrt(),
            Un::Cbrt => x.cbrt(),
            Un::Ln => x.ln(),
            Un::Log10 => x.log10(),
            Un::Log2 => x.log2(),
            Un::Fact => factorial(x),
            Un::Sin => snap(to_rad(x).sin()),
            Un::Cos => snap(to_rad(x).cos()),
            Un::Tan => {
                // tan(90°) is undefined.
                if snap(to_rad(x).cos()) == 0.0 { f64::NAN } else { snap(to_rad(x).tan()) }
            }
            Un::Asin => snap(from_rad(x.asin())),
            Un::Acos => snap(from_rad(x.acos())),
            Un::Atan => snap(from_rad(x.atan())),
            Un::Sinh => x.sinh(),
            Un::Cosh => x.cosh(),
            Un::Tanh => x.tanh(),
            Un::Asinh => x.asinh(),
            Un::Acosh => x.acosh(),
            Un::Atanh => x.atanh(),
        }
    }

    /// How the function is written in the expression line.
    fn label(self, inner: &str) -> String {
        match self {
            Un::Square => format!("{inner}²"),
            Un::Cube => format!("{inner}³"),
            Un::Exp => format!("e^{inner}"),
            Un::Pow10 => format!("10^{inner}"),
            Un::Pow2 => format!("2^{inner}"),
            Un::Recip => format!("1/{inner}"),
            Un::Sqrt => format!("√{inner}"),
            Un::Cbrt => format!("∛{inner}"),
            Un::Fact => format!("{inner}!"),
            Un::Ln => format!("ln({inner})"),
            Un::Log10 => format!("log({inner})"),
            Un::Log2 => format!("log₂({inner})"),
            Un::Sin => format!("sin({inner})"),
            Un::Cos => format!("cos({inner})"),
            Un::Tan => format!("tan({inner})"),
            Un::Asin => format!("sin⁻¹({inner})"),
            Un::Acos => format!("cos⁻¹({inner})"),
            Un::Atan => format!("tan⁻¹({inner})"),
            Un::Sinh => format!("sinh({inner})"),
            Un::Cosh => format!("cosh({inner})"),
            Un::Tanh => format!("tanh({inner})"),
            Un::Asinh => format!("sinh⁻¹({inner})"),
            Un::Acosh => format!("cosh⁻¹({inner})"),
            Un::Atanh => format!("tanh⁻¹({inner})"),
        }
    }
}

/// Rounds away tiny floating point noise, so sin(180°) shows 0 instead of 1.2e-16.
fn snap(x: f64) -> f64 {
    let r = x.round();
    if (x - r).abs() < 1e-12 { r } else { x }
}

/// n! for whole numbers, and the gamma function Γ(n+1) for everything else.
fn factorial(n: f64) -> f64 {
    if n < 0.0 && n.fract() == 0.0 {
        return f64::NAN;
    }
    if n.fract() == 0.0 && n <= 170.0 {
        return (1..=n as u64).fold(1.0, |acc, k| acc * k as f64);
    }
    gamma(n + 1.0)
}

/// Lanczos approximation of the gamma function.
fn gamma(x: f64) -> f64 {
    use std::f64::consts::PI;
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let t = x + 7.5;
    let sum = G[0] + (1..9).map(|i| G[i] / (x + i as f64)).sum::<f64>();
    (2.0 * PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * sum
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Const {
    Pi,
    E,
    Rand,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Input {
    Digit(char),
    Dot,
    Binary(Bin),
    Unary(Un),
    Const(Const),
    LParen,
    RParen,
    Equals,
    Clear,
    Backspace,
    Negate,
    Percent,
    MemClear,
    MemAdd,
    MemSub,
    MemRecall,
    ToggleAngle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Stack {
    Bin(Bin),
    Paren,
}

/// A calculator that respects operator precedence (2 + 3 × 4 = 14) and parentheses,
/// using the classic two-stack (shunting-yard) method as keys are pressed.
struct Engine {
    /// The number on screen.
    display: String,
    /// The next digit starts a new number instead of appending.
    fresh: bool,
    /// The number on screen is an operand the user entered (not a leftover).
    operand_ready: bool,
    /// How the operand on screen is written in the expression, e.g. "sin(30)".
    label: Option<String>,
    /// The last thing pressed was "=".
    just_evaluated: bool,
    values: Vec<f64>,
    ops: Vec<Stack>,
    /// The expression typed so far, as text pieces.
    expr: Vec<String>,
    /// The full expression of the last "=".
    last: String,
    error: bool,
    // These survive "C".
    memory: f64,
    radians: bool,
    seed: u64,
}

impl Default for Engine {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x2545_F491_4F6C_DD1D)
            | 1;
        Self {
            display: "0".into(),
            fresh: true,
            operand_ready: false,
            label: None,
            just_evaluated: false,
            values: Vec::new(),
            ops: Vec::new(),
            expr: Vec::new(),
            last: String::new(),
            error: false,
            memory: 0.0,
            radians: false,
            seed,
        }
    }
}

fn format_number(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    let a = n.abs();
    if !(1e-9..1e16).contains(&a) {
        let s = format!("{:.9e}", n);
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        return format!("{mantissa}e{exp}");
    }
    if n.fract() == 0.0 {
        return format!("{}", n as i64);
    }
    // Keep about 15 significant digits.
    let int_digits = (a.log10().floor() as i32 + 1).max(1);
    let decimals = (15 - int_digits).clamp(0, 12) as usize;
    let s = format!("{:.*}", decimals, n);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn render(tokens: &[String]) -> String {
    let mut out = String::new();
    for t in tokens {
        if !out.is_empty() && !out.ends_with('(') && t != ")" {
            out.push(' ');
        }
        out.push_str(t);
    }
    out
}

impl Engine {
    fn value(&self) -> f64 {
        self.display.parse().unwrap_or(0.0)
    }

    /// The operator waiting for its second number, if the user hasn't typed it yet.
    fn pending_operator(&self) -> Option<Bin> {
        match self.ops.last() {
            Some(Stack::Bin(op)) if !self.operand_ready => Some(*op),
            _ => None,
        }
    }

    fn operand_text(&mut self) -> String {
        self.label.take().unwrap_or_else(|| format_number(self.value()))
    }

    /// The small line shown above the main number.
    fn expression(&self) -> String {
        if self.expr.is_empty() { self.last.clone() } else { render(&self.expr) }
    }

    /// Puts a computed number on screen.
    fn show(&mut self, n: f64) {
        if n.is_finite() {
            self.display = format_number(n);
            self.fresh = true;
        } else {
            self.set_error();
        }
    }

    fn reset(&mut self) {
        let (memory, radians, seed) = (self.memory, self.radians, self.seed);
        *self = Self { memory, radians, seed, ..Self::default() };
    }

    fn set_error(&mut self) {
        self.reset();
        self.display = "Error".into();
        self.error = true;
    }

    /// Applies the operator on top of the stack. Returns false on error.
    fn reduce_one(&mut self) -> bool {
        let Some(Stack::Bin(op)) = self.ops.pop() else { return true };
        let b = self.values.pop().unwrap_or(0.0);
        let a = self.values.pop().unwrap_or(0.0);
        match op.apply(a, b) {
            Some(r) => {
                self.values.push(r);
                true
            }
            None => {
                self.set_error();
                false
            }
        }
    }

    /// Moves the operand on screen onto the value stack.
    fn push_operand(&mut self) {
        let v = self.value();
        let text = self.operand_text();
        self.values.push(v);
        self.expr.push(text);
    }

    fn set_operand(&mut self, n: f64, label: Option<String>) {
        self.show(n);
        if !self.error {
            self.label = label;
            self.operand_ready = true;
            self.just_evaluated = false;
        }
    }

    fn next_random(&mut self) -> f64 {
        // xorshift64
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }

    fn press(&mut self, input: Input) {
        if self.error {
            match input {
                Input::Clear | Input::Digit(_) | Input::Dot | Input::Const(_) | Input::MemRecall => self.reset(),
                Input::ToggleAngle | Input::MemClear => {}
                _ => return,
            }
        }

        match input {
            Input::Digit(d) => {
                if self.just_evaluated {
                    self.last.clear();
                }
                if self.fresh || self.display == "0" {
                    self.display = d.to_string();
                } else if self.display.len() < 16 {
                    self.display.push(d);
                }
                self.fresh = false;
                self.operand_ready = true;
                self.just_evaluated = false;
                self.label = None;
            }
            Input::Dot => {
                if self.fresh {
                    self.display = "0.".into();
                } else if !self.display.contains('.') && !self.display.contains('e') {
                    self.display.push('.');
                }
                self.fresh = false;
                self.operand_ready = true;
                self.just_evaluated = false;
                self.label = None;
            }
            Input::Binary(op) => {
                if self.operand_ready {
                    self.push_operand();
                } else if let Some(Stack::Bin(_)) = self.ops.last() {
                    // Pressing a different operator replaces the previous one.
                    self.ops.pop();
                    self.expr.pop();
                } else {
                    self.push_operand();
                }
                while let Some(Stack::Bin(top)) = self.ops.last().copied() {
                    let higher = top.precedence() > op.precedence()
                        || (top.precedence() == op.precedence() && !op.right_assoc());
                    if !higher {
                        break;
                    }
                    if !self.reduce_one() {
                        return;
                    }
                }
                if let Some(&v) = self.values.last() {
                    self.show(v);
                }
                self.ops.push(Stack::Bin(op));
                self.expr.push(op.symbol().into());
                self.operand_ready = false;
                self.just_evaluated = false;
                self.fresh = true;
            }
            Input::LParen => {
                // "2 (" means "2 × (".
                if self.operand_ready && !self.just_evaluated {
                    self.press(Input::Binary(Bin::Mul));
                    if self.error {
                        return;
                    }
                }
                self.ops.push(Stack::Paren);
                self.expr.push("(".into());
                self.operand_ready = false;
                self.just_evaluated = false;
                self.fresh = true;
            }
            Input::RParen => {
                if !self.ops.contains(&Stack::Paren) {
                    return;
                }
                self.push_operand();
                while let Some(Stack::Bin(_)) = self.ops.last() {
                    if !self.reduce_one() {
                        return;
                    }
                }
                self.ops.pop(); // the "("
                let open = self.expr.iter().rposition(|t| t == "(").unwrap_or(0);
                let inner = render(&self.expr[open + 1..]);
                self.expr.truncate(open);
                let v = self.values.pop().unwrap_or(0.0);
                self.set_operand(v, Some(format!("({inner})")));
            }
            Input::Equals => {
                if self.ops.is_empty() {
                    return;
                }
                self.push_operand();
                while let Some(top) = self.ops.last().copied() {
                    match top {
                        Stack::Bin(_) => {
                            if !self.reduce_one() {
                                return;
                            }
                        }
                        Stack::Paren => {
                            self.ops.pop();
                            self.expr.push(")".into());
                        }
                    }
                }
                let v = self.values.pop().unwrap_or(0.0);
                let text = format!("{} =", render(&self.expr));
                self.values.clear();
                self.expr.clear();
                self.set_operand(v, None);
                if !self.error {
                    self.last = text;
                    self.just_evaluated = true;
                }
            }
            Input::Unary(f) => {
                let x = self.value();
                let inner = self.operand_text();
                let label = f.label(&inner);
                self.set_operand(f.apply(x, self.radians), Some(label));
            }
            Input::Const(c) => {
                let (v, label) = match c {
                    Const::Pi => (std::f64::consts::PI, Some("π".to_string())),
                    Const::E => (std::f64::consts::E, Some("e".to_string())),
                    Const::Rand => (self.next_random(), None),
                };
                self.set_operand(v, label);
            }
            Input::Clear => self.reset(),
            Input::Backspace => {
                if !self.fresh {
                    self.display.pop();
                    if self.display.is_empty() || self.display == "-" {
                        self.display = "0".into();
                    }
                }
            }
            Input::Negate => {
                if !self.fresh {
                    // Still typing: just flip the sign of the text.
                    if let Some(rest) = self.display.strip_prefix('-') {
                        self.display = rest.to_string();
                    } else if self.display != "0" {
                        self.display.insert(0, '-');
                    }
                } else {
                    let label = self.label.take().map(|l| format!("−({l})"));
                    self.set_operand(-self.value(), label);
                }
            }
            Input::Percent => {
                // "200 + 10 %" means 10 % of 200, like a desk calculator.
                let x = self.value();
                let v = match (self.ops.last(), self.values.last()) {
                    (Some(Stack::Bin(Bin::Add | Bin::Sub)), Some(&base)) => base * x / 100.0,
                    _ => x / 100.0,
                };
                self.set_operand(v, None);
            }
            Input::MemClear => self.memory = 0.0,
            Input::MemAdd => {
                self.memory += self.value();
                self.fresh = true;
            }
            Input::MemSub => {
                self.memory -= self.value();
                self.fresh = true;
            }
            Input::MemRecall => self.set_operand(self.memory, None),
            Input::ToggleAngle => self.radians = !self.radians,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(c: &mut Engine, keys: &str) {
        for k in keys.chars() {
            let m = match k {
                '0'..='9' => Input::Digit(k),
                '.' => Input::Dot,
                '+' => Input::Binary(Bin::Add),
                '-' => Input::Binary(Bin::Sub),
                '*' => Input::Binary(Bin::Mul),
                '/' => Input::Binary(Bin::Div),
                '^' => Input::Binary(Bin::Pow),
                '(' => Input::LParen,
                ')' => Input::RParen,
                '!' => Input::Unary(Un::Fact),
                '=' => Input::Equals,
                'C' => Input::Clear,
                '<' => Input::Backspace,
                'n' => Input::Negate,
                '%' => Input::Percent,
                _ => panic!("unknown key {k}"),
            };
            c.press(m);
        }
    }

    fn eval(keys: &str) -> String {
        let mut c = Engine::default();
        press(&mut c, keys);
        c.display
    }

    fn run(inputs: &[Input]) -> Engine {
        let mut c = Engine::default();
        for &i in inputs {
            c.press(i);
        }
        c
    }

    #[test]
    fn basic_ops() {
        assert_eq!(eval("2+3="), "5");
        assert_eq!(eval("9-12="), "-3");
        assert_eq!(eval("6*7="), "42");
        assert_eq!(eval("7/2="), "3.5");
    }

    #[test]
    fn precedence() {
        assert_eq!(eval("2+3*4="), "14");
        assert_eq!(eval("1+2+3="), "6");
        assert_eq!(eval("2*3+4="), "10");
        assert_eq!(eval("2^3^2="), "512");
        assert_eq!(eval("10-2-3="), "5");
    }

    #[test]
    fn shows_running_result() {
        let mut c = Engine::default();
        press(&mut c, "2*3+");
        assert_eq!(c.display, "6");
    }

    #[test]
    fn parentheses() {
        assert_eq!(eval("(2+3)*4="), "20");
        assert_eq!(eval("2*(3+4)="), "14");
        assert_eq!(eval("2(3+4)="), "14");
        assert_eq!(eval("((1+2)*(3+4))="), "21");
        assert_eq!(eval("2*(3+4="), "14");
    }

    #[test]
    fn decimals_and_editing() {
        assert_eq!(eval("0.1+0.2="), "0.3");
        assert_eq!(eval("123<<"), "1");
        assert_eq!(eval("5n"), "-5");
        assert_eq!(eval("50%"), "0.5");
        assert_eq!(eval("200+10%="), "220");
        assert_eq!(eval(".5+.5="), "1");
    }

    #[test]
    fn divide_by_zero() {
        assert_eq!(eval("5/0="), "Error");
        assert_eq!(eval("5/0=C"), "0");
        assert_eq!(eval("5/0=3+1="), "4");
    }

    #[test]
    fn operator_change() {
        assert_eq!(eval("5+-3="), "2");
        assert_eq!(eval("2+3*+4="), "9");
    }

    #[test]
    fn continue_after_equals() {
        assert_eq!(eval("2+3=*2="), "10");
        assert_eq!(eval("2+3=7="), "7");
    }

    #[test]
    fn expression_line() {
        let mut c = Engine::default();
        press(&mut c, "12*3");
        assert_eq!(c.expression(), "12 ×");
        press(&mut c, "=");
        assert_eq!(c.expression(), "12 × 3 =");

        let mut c = Engine::default();
        press(&mut c, "(2+3)*4=");
        assert_eq!(c.expression(), "(2 + 3) × 4 =");
    }

    #[test]
    fn scientific_functions() {
        use Input::*;
        let d = |c| Digit(c);
        assert_eq!(run(&[d('3'), d('0'), Unary(Un::Sin)]).display, "0.5");
        assert_eq!(run(&[d('1'), d('8'), d('0'), Unary(Un::Sin)]).display, "0");
        assert_eq!(run(&[d('9'), Unary(Un::Sqrt)]).display, "3");
        assert_eq!(run(&[d('2'), d('7'), Unary(Un::Cbrt)]).display, "3");
        assert_eq!(run(&[d('5'), Unary(Un::Fact)]).display, "120");
        assert_eq!(run(&[d('2'), Binary(Bin::Pow), d('1'), d('0'), Equals]).display, "1024");
        assert_eq!(run(&[d('8'), Binary(Bin::Root), d('3'), Equals]).display, "2");
        assert_eq!(run(&[d('3'), Binary(Bin::EE), d('3'), Equals]).display, "3000");
        assert_eq!(run(&[d('8'), Binary(Bin::LogBase), d('2'), Equals]).display, "3");
        assert_eq!(run(&[d('2'), Binary(Bin::RevPow), d('3'), Equals]).display, "9");
        assert_eq!(run(&[d('1'), d('0'), d('0'), Unary(Un::Log10)]).display, "2");
        assert_eq!(run(&[d('4'), Unary(Un::Recip)]).display, "0.25");
        assert_eq!(run(&[d('9'), d('0'), Unary(Un::Tan)]).display, "Error");
        assert_eq!(run(&[d('2'), Unary(Un::Sqrt), Unary(Un::Square)]).display, "2");
    }

    #[test]
    fn radians_mode() {
        use Input::*;
        let c = run(&[ToggleAngle, Const(super::Const::Pi), Unary(Un::Cos)]);
        assert_eq!(c.display, "-1");
        assert!(c.radians);
        let c = run(&[ToggleAngle, Clear]);
        assert!(c.radians, "C keeps the angle mode");
    }

    #[test]
    fn memory() {
        use Input::*;
        let c = run(&[Digit('5'), MemAdd, Digit('3'), MemAdd, Clear, MemRecall]);
        assert_eq!(c.display, "8");
        let c = run(&[Digit('5'), MemAdd, Digit('2'), MemSub, MemClear, MemRecall]);
        assert_eq!(c.display, "0");
    }

    #[test]
    fn expression_with_functions() {
        use Input::*;
        let c = run(&[Digit('9'), Unary(Un::Sqrt), Binary(Bin::Add), Digit('1'), Equals]);
        assert_eq!(c.display, "4");
        assert_eq!(c.expression(), "√9 + 1 =");
    }

    #[test]
    fn formatting() {
        assert_eq!(format_number(1e20), "1e20");
        assert_eq!(format_number(1.5e-12), "1.5e-12");
        assert_eq!(format_number(1234.5), "1234.5");
        assert_eq!(format_number(-0.25), "-0.25");
        assert_eq!(format_number(1.0 / 3.0), "0.333333333333");
    }

    #[test]
    fn random_is_in_range() {
        let mut c = Engine::default();
        for _ in 0..100 {
            let r = c.next_random();
            assert!((0.0..1.0).contains(&r));
        }
    }

    #[test]
    fn keyboard_mapping() {
        assert!(matches!(input_from_key(&Key::Character("7".into())), Some(Input::Digit('7'))));
        assert!(matches!(input_from_key(&Key::Named(Named::Enter)), Some(Input::Equals)));
        assert!(matches!(input_from_key(&Key::Character("(".into())), Some(Input::LParen)));
        assert!(input_from_key(&Key::Character("q".into())).is_none());
    }

    #[test]
    fn pending_operator_is_highlighted_until_next_number() {
        let mut c = Engine::default();
        press(&mut c, "5*");
        assert_eq!(c.pending_operator(), Some(Bin::Mul));
        press(&mut c, "3");
        assert_eq!(c.pending_operator(), None);
    }

    #[test]
    fn size_setting() {
        let mut app = App::default();
        assert_eq!(app.size, UiSize::Normal);
        let _ = app.update(Message::SizeSelected(UiSize::Large));
        assert_eq!(app.size, UiSize::Large);
        assert!(UiSize::Compact.scale() < UiSize::Normal.scale());
        assert!(UiSize::Large.scale() > UiSize::Normal.scale());
    }

    #[test]
    fn themes() {
        let app = App::default();
        let mut names: Vec<_> = app.themes.iter().map(|t| t.name).collect();
        let total = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), total);
        assert_eq!(total, 7);
        assert!(app.current_theme().is_none(), "System is the default");
    }
}
