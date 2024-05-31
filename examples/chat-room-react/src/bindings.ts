import type { Stream } from "@qubit-rs/client";
export type ChatMessage = { user: string, content: string, };

export type QubitServer = { get_name: () => Promise<string>, send_message: (message: string) => Promise<null>, list_online: () => Stream<Array<string>>, list_messages: () => Stream<Array<ChatMessage>> };