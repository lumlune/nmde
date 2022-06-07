use {
    std::{
        future::Future,
        path::PathBuf,
        thread,
    },
    futures::executor,
    rfd::{
        AsyncFileDialog,
        FileHandle,
    },
};

type FileFilter = (&'static str, &'static[&'static str]);
enum FileAction {
    Open,
    Save,
}

fn execute<F>(routine: F)
    where F: Future<Output = ()> + Send + 'static,
{
    thread::spawn(|| {
        executor::block_on(routine);
    });
}

fn file_dialog<F>(action: FileAction, directory: &'static str, filters: &'static [FileFilter], callback: F)
    where F: FnOnce(PathBuf) + Send + 'static,
{
    execute(async {
        if let Some(file_handle) = file_dialog_internal(action, directory, filters).await {
            callback(file_handle.path().to_owned());
        }
    });
}

async fn file_dialog_internal(action: FileAction, directory: &str, filters: &[FileFilter]) -> Option<FileHandle> {
    let mut async_dialog = AsyncFileDialog::new()
        .set_directory(directory);

    for (name, extensions) in filters {
        async_dialog = async_dialog.add_filter(name, extensions);
    }

    match action {
        FileAction::Open => async_dialog.pick_file().await,
        FileAction::Save => async_dialog.save_file().await,
    }
}

pub fn open_file<F>(directory: &'static str, filters: &'static [FileFilter], callback: F)
    where F: FnOnce(PathBuf) + Send + 'static,
{
    file_dialog(FileAction::Open, directory, filters, callback);
}

pub fn save_file<F>(directory: &'static str, filters: &'static [FileFilter], callback: F)
    where F: FnOnce(PathBuf) + Send + 'static,
{
    file_dialog(FileAction::Save, directory, filters, callback);
}
