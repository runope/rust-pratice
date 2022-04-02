use anyhow::Result;
use tokio::sync::{mpsc, oneshot};

pub struct Actor<Request, Response, State> {
    receiver: mpsc::Receiver<Message<Request, Response>>,
    state: State,
}

impl<Request, Response, State> Actor<Request, Response, State>
where
    Request: HandleMessage<State = State, Response = Response> + Send + 'static,
    State: Default + Send + 'static,
    Response: Send + 'static,
{
    pub fn spawn(mailbox: usize) -> Addr<Request, Response> {
        let (sender, receiver) = mpsc::channel(mailbox);
        let mut actor: Actor<Request, Response, State> = Actor {
            receiver,
            state: Default::default(),
        };

        tokio::spawn(async move {
            while let Some(message) = actor.receiver.recv().await {
                let response = message.date.handle_message(&mut actor.state).unwrap();
                let _ = message.sender.send(response);
            }
        });

        Addr { sender }
    }
}

pub struct Message<Request, Response> {
    date: Request,
    sender: oneshot::Sender<Response>,
}

pub trait HandleMessage {
    type State;
    type Response;
    fn handle_message(&self, state: &mut Self::State) -> Result<Self::Response>;
}

#[derive(Clone)]
pub struct Addr<Request, Response> {
    sender: mpsc::Sender<Message<Request, Response>>,
}

impl<Request, Response> Addr<Request, Response> {
    pub async fn send(
        &self,
        date: Request,
    ) -> Result<Response, tokio::sync::oneshot::error::RecvError> {
        let (sender, receiver) = oneshot::channel();
        let _ = self.sender.send(Message { date, sender }).await;
        receiver.await
    }

    pub fn clone(&self) -> Addr<Request, Response> {
        Addr {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time;

    use super::*;

    struct Req(i32);
    #[derive(Debug, PartialEq)]    
    struct Res(i32);

    impl HandleMessage for Req {
        type State = (u32,);

        

        type Response = Res;

        fn handle_message(&self, state: &mut Self::State) -> Result<Self::Response> {
            state.0 += 1;
            println!("state: {}", state.0);
            Ok(Res(self.0 + 1))
        }
    }

    #[tokio::test]
    async fn it_works() {
        let addr: Addr<Req, Res> = Actor::spawn(10);
        let addr2 = addr.clone();

        tokio::spawn(async move {
            let ret = addr.send(Req(1)).await.unwrap();
            println!("ret1: {:?}", ret);
            assert_eq!(ret, Res(2));
        });
        
        tokio::spawn(async move {
            let ret = addr2.send(Req(100)).await.unwrap();
            println!("ret1: {:?}", ret);
            assert_eq!(ret, Res(101));
        });

        tokio::time::sleep(time::Duration::from_secs(1)).await;
    }
}