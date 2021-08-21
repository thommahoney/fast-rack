use fastly::{Error, Request, Response};

/// MAX_RETRIES
pub const MAX_RETRIES: u8 = 3;

const X_RETRIES_HEADER: &str = "x-retries";

/// FastRack
pub struct FastRack<'rack> {
    pub middleware: Vec<&'rack mut dyn Middleware>,
}

impl<'rack> FastRack<'rack> {
    /// new
    pub fn new() -> Self {
        FastRack {
            middleware: Vec::<&mut dyn Middleware>::new(),
        }
    }

    /// add
    pub fn add(&mut self, middleware: &'rack mut (dyn Middleware + 'rack)) {
        self.middleware.push(middleware);
    }

    /// run
    pub fn run(&mut self, request: &mut Request) -> Result<Response, Error> {
        let mut response = Response::new();
        let mut retries = 0;

        loop {
            match self.run_inner(request, &mut response) {
                Ok(()) => break,
                Err(e) => match e {
                    RackError::Retry => {
                        retries += 1;

                        if retries >= MAX_RETRIES {
                            break
                        }
                    },
                    RackError::Synthetic(resp) => {
                        response = resp;
                        break
                    },
                }
            }
        }

        if retries > 0 {
            response.set_header(X_RETRIES_HEADER, format!("{}", retries));
        }

        Ok(response)
    }

    fn run_inner(&mut self, request: &mut Request, response: &mut Response) -> Result<(), RackError> {
        for m in self.middleware.iter_mut() {
            m.req(request)?;
        }

        for m in self.middleware.iter_mut().rev() {
            m.resp(response)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum RackError {
    Retry,
    Synthetic(Response),
}

/// Middleware
pub trait Middleware {
    /// req
    fn req(&mut self, req: &mut Request) -> Result<(), RackError>;

    /// resp
    fn resp(&mut self, resp: &mut Response) -> Result<(), RackError>;
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::OpenOptions;
    use std::sync::atomic::AtomicU16;
    use std::sync::atomic::Ordering::SeqCst;

    use tokio::process::{Child, Command};
    use reqwest;

    static ATOMIC_PORT: AtomicU16 = AtomicU16::new(7878);
    static FAST_RACK_TEST_DEBUG: &'static str = "FAST_RACK_TEST_DEBUG";

    async fn setup(test_name: &str) -> (Child, u16) {
        let build_output = Command::new("cargo")
            .args(&["build", "--target=wasm32-wasi"])
            .current_dir(format!("test/{}", test_name))
            .kill_on_drop(true)
            .output().await
            .expect("setup failure: failed to build");

        let viceroy_stdout_path = format!("test/debug/{}.viceroy.stdout", test_name);
        let viceroy_stderr_path = format!("test/debug/{}.viceroy.stderr", test_name);

        if let Some(_s) = env::var_os(FAST_RACK_TEST_DEBUG) {
            println!("=== {} stdout ===\n\n{}", FAST_RACK_TEST_DEBUG, String::from_utf8_lossy(&build_output.stdout));
            println!("=== {} stderr ===\n\n{}", FAST_RACK_TEST_DEBUG, String::from_utf8_lossy(&build_output.stderr));
            println!("=== {} ===\n\nMore debug output may be available in:\n    {}\n    {}\n", FAST_RACK_TEST_DEBUG, viceroy_stdout_path, viceroy_stderr_path);
        }

        let port = ATOMIC_PORT.fetch_add(1, SeqCst);

        let viceroy_stdout = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(viceroy_stdout_path)
            .expect("failed to open viceroy stdout");
        let viceroy_stderr = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(viceroy_stderr_path)
            .expect("failed to open viceroy stderr");

        let child = Command::new("viceroy")
            .arg("--addr")
            .arg(format!("127.0.0.1:{}", port))
            .arg(format!("test/{}/target/wasm32-wasi/debug/{}.wasm", test_name, test_name))
            .stdout(viceroy_stdout)
            .stderr(viceroy_stderr)
            .kill_on_drop(true)
            .spawn()
            .expect("setup failure: failed to start viceroy");

        // TODO: TCP connect to viceroy?
        tokio::time::sleep(std::time::Duration::new(1, 0)).await;

        (child, port)
    }

    async fn teardown(child: &mut Child) {
        child.kill().await.expect("teardown failure")
    }

    #[tokio::test]
    async fn it_responds_with_a_synthetic() {
        let (mut child, port) = setup("synthetic-response").await;

        let resp = reqwest::get(format!("http://127.0.0.1:{}/", port)).await.expect("reqwest::get failure");

        assert_eq!(418, resp.status());
        assert_eq!("foo", resp.text().await.expect("failed to read response body"));

        teardown(&mut child).await;
    }

    #[tokio::test]
    async fn it_retries_requests() {
        let (mut child, port) = setup("retry-request").await;

        let resp = reqwest::get(format!("http://127.0.0.1:{}/", port)).await.expect("reqwest::get failure");

        assert_eq!(200, resp.status());
        let retries_header = resp.headers().get("x-retries").expect("retries header missing");
        assert_eq!("2", retries_header);

        teardown(&mut child).await;
    }
}
