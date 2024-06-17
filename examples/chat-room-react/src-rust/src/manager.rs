use futures::{stream, Stream};
use serde::Serialize;
use tokio::sync::mpsc;
use ts_rs::TS;

#[derive(Clone, Serialize, TS)]
pub struct ChatMessage {
    user: char,
    content: String,
}

#[derive(Clone, Default)]
pub struct Manager {
    users: Vec<char>,
    messages: Vec<ChatMessage>,
    subscriptions: Subscriptions,
}

pub enum Message {
    /// Add a user to the list
    #[allow(dead_code)]
    Join { name: char },

    /// Remove a user from the list
    #[allow(dead_code)]
    Leave { name: char },

    /// Send a chat message
    Send { user: char, message: String },

    /// Subscribe to a list of online users
    RegisterOnline { tx: mpsc::Sender<Vec<char>> },

    /// Subscribe to a list of messages
    RegisterMessages { tx: mpsc::Sender<Vec<ChatMessage>> },
}

#[derive(Clone)]
pub struct Client {
    tx: mpsc::Sender<Message>,
}

impl Client {
    pub fn new(tx: mpsc::Sender<Message>) -> Self {
        Self { tx }
    }

    #[allow(dead_code)]
    pub async fn join(&self, name: char) {
        self.tx.send(Message::Join { name }).await.unwrap();
    }

    #[allow(dead_code)]
    pub async fn leave(&self, name: char) {
        self.tx.send(Message::Leave { name }).await.unwrap();
    }

    pub async fn send_message(&self, user: char, message: String) {
        self.tx.send(Message::Send { user, message }).await.unwrap();
    }

    pub async fn stream_online(&self) -> impl Stream<Item = Vec<char>> {
        let (tx, rx) = mpsc::channel(10);
        self.tx.send(Message::RegisterOnline { tx }).await.unwrap();
        stream::unfold(rx, |mut rx| async move { Some((rx.recv().await?, rx)) })
    }

    pub async fn stream_messages(&self) -> impl Stream<Item = Vec<ChatMessage>> {
        let (tx, rx) = mpsc::channel(10);
        self.tx
            .send(Message::RegisterMessages { tx })
            .await
            .unwrap();
        stream::unfold(rx, |mut rx| async move { Some((rx.recv().await?, rx)) })
    }
}

impl Manager {
    pub fn start() -> Client {
        let (tx, mut rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let mut manager = Manager::default();

            while let Some(message) = rx.recv().await {
                manager.process(message).await;
            }
        });

        Client::new(tx)
    }

    async fn process(&mut self, message: Message) {
        match message {
            Message::Join { name } => {
                self.join(name).await;
            }
            Message::Leave { name } => {
                self.leave(name).await;
            }
            Message::Send { user, message } => {
                self.send_message(user, message).await;
            }
            Message::RegisterOnline { tx } => {
                self.register_online_subscription(tx).await;
            }
            Message::RegisterMessages { tx } => {
                self.register_messages_subscription(tx).await;
            }
        }
    }

    async fn join(&mut self, name: char) {
        self.users.push(name);
        self.subscriptions.update_register_online(&self.users).await;
    }

    async fn leave(&mut self, name: char) {
        self.users.retain(|c| *c != name);
        self.subscriptions.update_register_online(&self.users).await;
    }

    async fn send_message(&mut self, user: char, message: String) {
        self.messages.push(ChatMessage {
            user,
            content: message,
        });
        self.subscriptions
            .update_register_messages(&self.messages)
            .await;
    }

    async fn register_online_subscription(&mut self, tx: mpsc::Sender<Vec<char>>) {
        self.subscriptions
            .register_online(tx, self.users.clone())
            .await;
    }

    async fn register_messages_subscription(&mut self, tx: mpsc::Sender<Vec<ChatMessage>>) {
        self.subscriptions
            .register_messages(tx, self.messages.clone())
            .await;
    }
}

#[derive(Default, Clone)]
struct Subscriptions {
    online: Vec<mpsc::Sender<Vec<char>>>,
    messages: Vec<mpsc::Sender<Vec<ChatMessage>>>,
}

impl Subscriptions {
    async fn register_online(&mut self, tx: mpsc::Sender<Vec<char>>, users: Vec<char>) {
        if tx.send(users).await.is_ok() {
            self.online.push(tx);
        }
    }

    async fn update_register_online(&mut self, users: &[char]) {
        self.online.retain(|tx| !tx.is_closed());
        for tx in self.online.iter() {
            tx.send(users.to_vec()).await.unwrap();
        }
    }

    async fn register_messages(
        &mut self,
        tx: mpsc::Sender<Vec<ChatMessage>>,
        messages: Vec<ChatMessage>,
    ) {
        if tx.send(messages).await.is_ok() {
            self.messages.push(tx);
        }
    }

    async fn update_register_messages(&mut self, messages: &[ChatMessage]) {
        self.messages.retain(|tx| !tx.is_closed());
        for tx in self.messages.iter() {
            tx.send(messages.to_vec()).await.unwrap();
        }
    }
}
