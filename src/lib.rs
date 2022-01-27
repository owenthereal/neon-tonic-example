mod function;

use function::*;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};

fn start_func(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let f = cx.argument::<JsFunction>(0)?.root(&mut cx);
    let channel = cx.channel();

    let addr = "[::1]:50051".parse().unwrap();
    let func = Function { ch: channel };

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

pub struct Function {
    ch: Channel,
}

#[tonic::async_trait]
impl function_server::Function for Function {
    async fn process(
        &self,
        request: Request<FunctionRequest>,
    ) -> Result<Response<FunctionResponse>, Status> {
        println!("Got a request: {:?}", request);

        let value = request.into_inner().value;
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.ch.send(move |mut cx| {
            // QUESTION: How would I pass `f` here?
            let f = f.into_inner(&mut cx);
            let this = cx.undefined();
            let value: Handle<JsString> = f.call(&mut cx, this, [])?;
            sender.send(value.value(&mut cx));
        });

        let value = receiver.recv().await.unwrap();
        let reply = FunctionResponse { value: value };

        Ok(Response::new(reply))
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("startFunc", start_func)?;
    Ok(())
}
