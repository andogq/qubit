import { useQuery } from '@tanstack/react-query'
import { Message } from './Message'
import { api } from '../api'
import { useEffect, useRef, useState } from 'react'
import { ChatMessage } from '../bindings'

export const History = () => {
  const containerRef = useRef<HTMLOutputElement>(null)

  const { data: name } = useQuery({
    queryKey: ['name'],
    queryFn: () => api.get_name()
  })

  const [messages, setMessages] = useState<ChatMessage[]>([])

  useEffect(() => api.list_messages().subscribe({ on_data: setMessages }), [])

  useEffect(() => {
    containerRef.current?.scrollTo({ top: containerRef.current.scrollHeight, behavior: 'smooth' })
  }, [messages])

  return <output ref={containerRef}>
    {messages.map((message, i) => <Message key={i} emoji={message.user} message={message.content} you={message.user === name} />)}
  </output>
}
