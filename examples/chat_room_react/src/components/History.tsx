import { useQuery } from '@tanstack/react-query'
import { Message } from './Message'
import { api } from '../api'
import { useEffect, useState } from 'react'
import { ChatMessage } from '../bindings'

export const History = () => {
  const { data: name } = useQuery({
    queryKey: ['name'],
    queryFn: () => api.get_name()
  })

  const [messages, setMessages] = useState<ChatMessage[]>([])

  useEffect(() => api.list_messages().subscribe({ on_data: setMessages }), [])

  return <output>
    {messages.map((message, i) => <Message key={i} emoji={message.user} message={message.content} you={message.user === name} />)}
  </output>
}
