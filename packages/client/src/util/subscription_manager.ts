import type { RpcSubscriptionMessage } from "../jsonrpc";

type SubscriptionHandler = (value: unknown) => void;
type Subscription = { queue: unknown[]; handler?: SubscriptionHandler };

/**
 * Collects incoming messages until a handler is registered for them. Once a handler is
 * registered, all previous messages will be replayed.
 */
export function create_subscription_manager() {
  const subscriptions: Record<string | number, Subscription> = {};

  function get_subscription(id: string | number): Subscription {
    let subscription = subscriptions[id];
    if (!subscription) {
      subscription = subscriptions[id] = { queue: [] };
    }

    return subscription;
  }

  return {
    /** Handle an incoming message */
    handle: (message: RpcSubscriptionMessage<unknown>) => {
      // Fetch the subscription (creating it if it doesn't exist)
      const subscription = get_subscription(message.id);

      // If the handler doesn't exist, save it for later
      if (!subscription.handler) {
        subscription.queue.push(message.value);
        return;
      }

      // Pass the message value on to the handler
      subscription.handler(message.value);
    },

    /** Register a handler for when new data arrives for a given subscription. */
    register: (id: string | number, handler: SubscriptionHandler) => {
      const subscription = get_subscription(id);

      // Make sure the handler won't be over written
      if (subscription.handler) {
        console.error(`attempted to subscribe to a subscription multiple times (subscription ID: ${id})`);
        return;
      }

      // Save the handler
      subscription.handler = handler;

      // Empty out anything that's currently in the queue
      for (const value of subscription.queue) {
        handler(value);
      }
      subscription.queue = [];
    },

    /** Remove the given subscription. */
    remove: (id: string | number) => {
      delete subscriptions[id];
    },
  };
}
