import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { History } from "./components/History";
import { Input } from "./components/Input";
import { Online } from "./components/Online";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: false },
  },
});

export const App = () => {
  return (
    <QueryClientProvider client={queryClient}>
      <h1>Chat Room</h1>

      <main>
        <History />

        <Input />
      </main>

      <Online />
    </QueryClientProvider>
  );
};
