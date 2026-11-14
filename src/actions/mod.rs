use crate::models::Item;
use gio::prelude::AppInfoExt;
use std::error::Error;

pub fn launch(item: &Item) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Item::App(app) = item;
    let info = gio::DesktopAppInfo::from_filename(&app.app_desktop_path).ok_or_else(|| {
        format!(
            "No se pudo cargar el fichero .desktop: {:?}",
            app.app_desktop_path
        )
    })?;
    info.launch_uris(&[], None::<&gio::AppLaunchContext>)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::actions::launch;
    use crate::models::Item;
    use applications::App;
    use std::path::PathBuf;

    #[test]
    fn launch_missing_desktop_returns_err() {
        let item = Item::App(App {
            name: "Fantasma".to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: PathBuf::from("/tmp/no-existe-jamas/ghost.desktop"),
        });
        assert!(launch(&item).is_err());
    }
}
