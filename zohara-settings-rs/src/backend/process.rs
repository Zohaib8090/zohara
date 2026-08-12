/// Run a shell command asynchronously and return stdout as a String.
/// Never blocks the GTK main loop — spawns on glib's async executor.
use glib;

pub fn run_async<F>(cmd: &'static str, args: Vec<String>, callback: F)
where
    F: Fn(Result<String, String>) + 'static,
{
    glib::spawn_future_local(async move {
        let result = tokio::process::Command::new(cmd)
            .args(&args)
            .output()
            .await;

        match result {
            Ok(out) if out.status.success() => {
                callback(Ok(String::from_utf8_lossy(&out.stdout).into_owned()))
            }
            Ok(out) => {
                callback(Err(String::from_utf8_lossy(&out.stderr).into_owned()))
            }
            Err(e) => callback(Err(e.to_string())),
        }
    });
}
