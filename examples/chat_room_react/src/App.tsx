import { Avatar } from './components/Avatar'
import { Message } from './components/Message'

export const App = () => {
  return (
    <>
      <h1>Chat Room</h1>

      <main>
        <output>
          <Message emoji="🦀" message="Hi there! Hi there! Hi there! Hi there! Hi there! Hi there!" />
          <Message emoji="🌼" message="Hi there! Hi there! Hi there! Hi there! Hi there! Hi there!" />
          <Message emoji="⚠️" message="Hi there! Hi there! Hi there! Hi there! Hi there! Hi there!" />
          <Message emoji="🥔" message="Hi there! Hi there! Hi there! Hi there! Hi there! Hi there!" you />
        </output>

        <form>
          <Avatar emoji="🥔" />
          <input placeholder="Enter a message" />
          <button>Send</button>
        </form>
      </main>

      <section>
        <h2>Online (4)</h2>

        <div id="online">
          <Avatar emoji="🌼" />
          <Avatar emoji="🦀" />
          <Avatar emoji="⚠️" />
          <Avatar emoji="🥔" />
        </div>
      </section>
    </>
  )
}
