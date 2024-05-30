import { useEffect, useState } from "react";
import { api } from "../api";
import { Avatar } from "./Avatar";

export const Online = () => {
  const [users, setUsers] = useState<string[]>([]);

  useEffect(() => api.list_online().subscribe({ on_data: setUsers }), []);

  return (
    <section>
      <h2>Online ({users.length})</h2>

      <div id="online">
        {users.map((user) => (
          <Avatar key={user} emoji={user} />
        ))}
      </div>
    </section>
  );
};
