import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import { Avatar } from './Avatar'

export const Input = () => {
  const { data: name } = useQuery({
    queryKey: ['name'],
    queryFn: () => api.get_name()
  })

  return <form>
    <Avatar emoji={name ?? '?'} />
    <input placeholder="Enter a message" />
    <button>Send</button>
  </form>
}
