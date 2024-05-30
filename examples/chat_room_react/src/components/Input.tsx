import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import { Avatar } from './Avatar'
import { useState } from 'react'

export const Input = () => {
  const [value, setValue] = useState('')

  const { data: name } = useQuery({
    queryKey: ['name'],
    queryFn: () => api.get_name()
  })

  return <form onSubmit={e => {
    e.preventDefault()
    const message = value.trim()
    if (value.length > 0) {
      api.send_message(message)
      setValue('')
    }
  }}>
    <Avatar emoji={name ?? '?'} />
    <input placeholder="Enter a message" value={value} onChange={e => setValue(e.target.value)} />
    <button>Send</button>
  </form>
}
