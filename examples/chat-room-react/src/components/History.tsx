import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { ChatMessage } from "../bindings/ChatMessage";
import { Message } from "./Message";

export const History = () => {
  const containerRef = useRef<HTMLOutputElement>(null);

  const { data: name } = useQuery({
    queryKey: ["name"],
    queryFn: () => api.get_name.query(),
  });

  const [messages, setMessages] = useState<ChatMessage[]>([]);

  useEffect(() => api.list_messages.subscribe({ on_data: setMessages }), []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: scroll when messages changes
  useEffect(() => {
    containerRef.current?.scrollTo({
      top: containerRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, [messages]);

  return (
    <output ref={containerRef}>
      {messages.map((message, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: order will never change
        <Message
          key={i}
          emoji={message.user}
          message={message.content}
          you={message.user === name}
        />
      ))}
    </output>
  );
};
