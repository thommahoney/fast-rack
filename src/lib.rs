use fastly::{Error, Request, Response};

/// FastRack
pub struct FastRack<'rack> {
    pub middleware: Vec<&'rack dyn Middleware>,
}

impl<'rack> FastRack<'rack> {
    /// new
    pub fn new() -> Self {
        FastRack {
            middleware: Vec::<&dyn Middleware>::new(),
        }
    }

    /// add
    pub fn add(&mut self, middleware: &'rack (dyn Middleware + 'rack)) {
        self.middleware.push(middleware);
    }

    /// run
    pub fn run(&self, request: &mut Request) -> Result<Response, Error> {
        let mut response = Response::new();

        for m in self.middleware.iter() {
            match m.req(request) {
                Ok(()) => {},
                Err(e) => match e {
                    RackError::Synthetic(resp) => {
                        response = resp;
                        break; // halt further execution
                    },
                }
            }
        }

        for m in self.middleware.iter().rev() {
            match m.resp(&mut response) {
                Ok(()) => {},
                Err(_e) => {
                    todo!()
                }
            }
        }

        Ok(response)
    }
}

#[derive(Debug)]
pub enum RackError {
    Synthetic(Response),
}

/// Middleware
pub trait Middleware {
    /// req
    fn req(&self, req: &mut Request) -> Result<(), RackError>;

    /// resp
    fn resp(&self, resp: &mut Response) -> Result<(), RackError>;
}

#[cfg(test)]
mod tests {
    use tokio::process::{Child, Command};
    use reqwest;

    async fn setup(test_name: &str) -> Child {
        Command::new("cargo")
            .current_dir(format!("test/{}", test_name))
            .args(&["build", "--target=wasm32-wasi"])
            .spawn()
            .expect("setup failure: failed to spawn cargo build")
            .wait().await
            .expect("setup failure: failed to build");

        let child = Command::new("viceroy")
            .args(&[format!("test/{}/target/wasm32-wasi/debug/{}.wasm", test_name, test_name).as_str()])
            .kill_on_drop(true)
            .spawn()
            .expect("setup failure: failed to start viceroy");

        // TODO: TCP connect to viceroy?
        tokio::time::sleep(std::time::Duration::new(1, 0)).await;

        child
    }

    async fn teardown(child: &mut Child) {
        child.kill().await.expect("teardown failure")
    }

    #[tokio::test]
    async fn it_responds_with_a_synthetic() {
        let mut child = setup("synthetic-response").await;

        let resp = reqwest::get("http://127.0.0.1:7878/").await.expect("reqwest::get failure");

        assert_eq!(418, resp.status());
        assert_eq!("foo", resp.text().await.expect("failed to read response body"));

        teardown(&mut child).await;
    }
}
