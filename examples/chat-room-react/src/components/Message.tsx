import { Avatar } from './Avatar'

export const Message = ({ emoji, message, you }: { emoji: string, message: string, you?: boolean }) => {
  return <div className={`message ${you ? 'you' : ''}`}>
    <Avatar emoji={emoji} />
    <span>{message}</span>
  </div>
}
