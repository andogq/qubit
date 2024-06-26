import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../api";
import { Avatar } from "./Avatar";

export const Input = () => {
  const [value, setValue] = useState("");

  const { data: name } = useQuery({
    queryKey: ["name"],
    queryFn: () => api.get_name.query(),
  });

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const message = value.trim();
        if (value.length > 0) {
          api.send_message.mutate(message);
          setValue("");
        }
      }}
    >
      <Avatar emoji={name ?? "?"} />
      <input placeholder="Enter a message" value={value} onChange={(e) => setValue(e.target.value)} />
      <button type="submit">Send</button>
    </form>
  );
};
