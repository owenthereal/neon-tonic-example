mod function;

use function::*;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};

fn start_func(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);

    // how to pass this callback into FunctionImpl and call the function in javascript
    // see CALL_CALLBACK below

    let addr = "[::1]:50051".parse().unwrap();
    let func = Function {};

    let rt = runtime(&mut cx)?;
    let result = rt.block_on(
        Server::builder()
            .add_service(function_server::FunctionServer::new(func))
            .serve(addr),
    );

    Ok(cx.undefined())
}

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

pub struct Function {}

#[tonic::async_trait]
impl function_server::Function for Function {
    async fn process(
        &self,
        request: Request<FunctionRequest>,
    ) -> Result<Response<FunctionResponse>, Status> {
        println!("Got a request: {:?}", request);

        let value = request.into_inner().value;

        // CALL_CALLBACK by passing in records

        let reply = FunctionResponse { value: value };

        Ok(Response::new(reply))
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("startFunc", start_func)?;
    Ok(())
}