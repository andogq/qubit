import type { Stream } from "@qubit-rs/client";
export type ChatMessage = { user: string, content: string, };

export type Server = { get_name: () => Promise<string>, list_online: () => Stream<Array<string>>, list_messages: () => Stream<Array<ChatMessage>> };