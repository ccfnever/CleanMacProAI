use std::path::Path;

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    let mut context = trash::TrashContext::default();

    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }

    context.delete(path).map_err(|error| error.to_string())
}
