//! Built-in online lobby uses the same GPU screen controls as authored screens.
use rusterix::{Rect, Sector, Value};
use shared::{project::Project, screen::Screen};
pub const ACCOUNT: &str = "__eldiron_account_settings";
pub const NAME: &str = "__eldiron_online_lobby";
pub fn install(project: &mut Project) {
    let mut screen = Screen::new();
    screen.name = NAME.into();
    screen.map.name = NAME.into();
    screen.settings = "[layout]\nmode='responsive'".into();
    let config = eldiron_runtime::PlayerSettings::parse(&project.config).unwrap();
    let width = config.viewport.0 as f32;
    let height = config.viewport.1 as f32;
    let x = (width - 440.).max(0.) / 2.;
    let y = (height - 470.).max(0.) / 2.;
    let (sx, sy) = rusterix::utils::align_screen_to_grid(width, height, config.grid_size);
    screen.map.subdivisions = config.grid_size * 2.;
    let mut add = |id, name: &str, r: Rect, data: &str| {
        let points = [
            (r.x, r.y),
            (r.x + r.width, r.y),
            (r.x + r.width, r.y + r.height),
            (r.x, r.y + r.height),
        ]
        .map(|(x, y)| {
            screen
                .map
                .add_vertex_at(x / config.grid_size + sx, y / config.grid_size + sy)
        });
        let lines = (0..4)
            .map(|i| {
                screen
                    .map
                    .create_linedef_manual(points[i], points[(i + 1) % 4])
            })
            .collect();
        let mut sector = Sector::new(id, lines);
        sector.name = name.into();
        sector.properties.set("data", Value::Str(data.into()));
        screen.map.sectors.push(sector);
    };
    add(
        1,
        "OnlineTitle",
        Rect::new(x, y, 440., 30.),
        "[ui]\nrole='text'\ntext='Connect to game'\nfont_size=26.0",
    );
    add(
        2,
        "OnlineStatus",
        Rect::new(x, y + 34., 440., 40.),
        "[ui]\nrole='text'\ntext='Connecting…'\nfont_size=16.0",
    );
    for (id, name, label, dy, bind, secret, default) in [
        (
            3,
            "OnlineUsername",
            "Username",
            80.,
            "online.username",
            false,
            "",
        ),
        (
            5,
            "OnlinePassword",
            "Password",
            136.,
            "online.password",
            true,
            "",
        ),
        (
            7,
            "OnlineName",
            "Character name",
            192.,
            "start.name",
            false,
            "Adventurer",
        ),
    ] {
        add(
            id,
            &format!("{name}Label"),
            Rect::new(x, y + dy, 440., 22.),
            &format!("[ui]\nrole='text'\ntext='{label}'\nfont_size=16.0"),
        );
        add(
            id + 1,
            name,
            Rect::new(x, y + dy + 23., 440., 30.),
            &format!(
                "[ui]\nrole='input'\nbind='{bind}'\ndefault='{default}'\nsecret={secret}\nfont_size=19.0"
            ),
        );
    }
    for (id, name, dy, command, label) in [
        (9, "OnlineLogin", 252., "game.login", "Log in"),
        (
            10,
            "OnlineRegister",
            292.,
            "game.register",
            "Create account",
        ),
        (11, "OnlinePlay", 332., "game.start", "Play"),
        (
            12,
            "OnlineNext",
            372.,
            "game.next_character",
            "Next character",
        ),
        (
            13,
            "OnlineCreate",
            412.,
            "game.create_character",
            "New character",
        ),
        (14, "OnlineReconnect", 452., "game.connect", "Reconnect"),
    ] {
        add(
            id,
            name,
            Rect::new(x, y + dy, if id == 14 { 216. } else { 440. }, 34.),
            &format!("[ui]\nrole='button'\ncommand='{command}'\nlabel='{label}'\nshow_icon=false"),
        );
    }
    add(
        15,
        "OnlineAccount",
        Rect::new(x + 224., y + 452., 216., 34.),
        "[ui]\nrole='button'\ncommand='game.account'\nlabel='Account'\nshow_icon=false",
    );
    project.screens.insert(screen.id, screen);
    install_account(project);
}

fn install_account(project: &mut Project) {
    let mut screen = Screen::new();
    screen.name = ACCOUNT.into();
    screen.map.name = ACCOUNT.into();
    screen.settings = "[layout]\nmode='responsive'".into();
    let c = eldiron_runtime::PlayerSettings::parse(&project.config).unwrap();
    let w = c.viewport.0 as f32;
    let h = c.viewport.1 as f32;
    let x = (w - 440.).max(0.) / 2.;
    let y = (h - 470.).max(0.) / 2.;
    let (sx, sy) = rusterix::utils::align_screen_to_grid(w, h, c.grid_size);
    screen.map.subdivisions = c.grid_size * 2.;
    let mut add = |id, name: &str, dy: f32, height: f32, data: String| {
        let r = Rect::new(x, y + dy, 440., height);
        let points = [
            (r.x, r.y),
            (r.x + r.width, r.y),
            (r.x + r.width, r.y + r.height),
            (r.x, r.y + r.height),
        ]
        .map(|(a, b)| {
            screen
                .map
                .add_vertex_at(a / c.grid_size + sx, b / c.grid_size + sy)
        });
        let lines = (0..4)
            .map(|i| {
                screen
                    .map
                    .create_linedef_manual(points[i], points[(i + 1) % 4])
            })
            .collect();
        let mut s = Sector::new(id, lines);
        s.name = name.into();
        s.properties.set("data", Value::Str(data));
        screen.map.sectors.push(s);
    };
    add(
        1,
        "AccountTitle",
        0.,
        30.,
        "[ui]\nrole='text'\ntext='Account settings'\nfont_size=26.0".into(),
    );
    add(
        2,
        "OnlineStatus",
        34.,
        40.,
        "[ui]\nrole='text'\ntext=''\nfont_size=16.0".into(),
    );
    for (id, name, label, dy, bind) in [
        (
            3,
            "CurrentPassword",
            "Current password",
            80.,
            "online.current_password",
        ),
        (
            5,
            "NewPassword",
            "New password (12–256 bytes)",
            136.,
            "online.new_password",
        ),
        (
            7,
            "ConfirmPassword",
            "Confirm new password",
            192.,
            "online.confirm_password",
        ),
    ] {
        add(
            id,
            &format!("{name}Label"),
            dy,
            22.,
            format!("[ui]\nrole='text'\ntext='{label}'\nfont_size=16.0"),
        );
        add(
            id + 1,
            name,
            dy + 23.,
            30.,
            format!("[ui]\nrole='input'\nbind='{bind}'\nsecret=true\nfont_size=19.0"),
        );
    }
    for (id, name, dy, command, label) in [
        (
            9,
            "ChangePassword",
            252.,
            "change_password",
            "Change password and sign out",
        ),
        (
            10,
            "LogoutEverywhere",
            292.,
            "logout_all",
            "Log out everywhere",
        ),
        (11, "Logout", 332., "logout", "Log out"),
        (12, "AccountBack", 372., "account_back", "Back"),
    ] {
        add(
            id,
            name,
            dy,
            34.,
            format!(
                "[ui]\nrole='button'\ncommand='game.{command}'\nlabel='{label}'\nshow_icon=false"
            ),
        );
    }
    project.screens.insert(screen.id, screen);
}
