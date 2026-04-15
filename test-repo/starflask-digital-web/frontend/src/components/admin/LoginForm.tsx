import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { toast } from 'sonner'
import { api } from '../../lib/api'
import Button from '../ui/Button'
import Input from '../ui/Input'

interface LoginFormProps {
  onSuccess: () => void
}

export default function LoginForm({ onSuccess }: LoginFormProps) {
  const [loading, setLoading] = useState(false)
  const { register, handleSubmit } = useForm<{ password: string }>()

  const onSubmit = async ({ password }: { password: string }) => {
    setLoading(true)
    try {
      await api.adminLogin(password)
      onSuccess()
    } catch {
      toast.error('Invalid password')
    } finally {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="max-w-sm mx-auto space-y-6">
      <Input
        label="Admin Password"
        type="password"
        placeholder="Enter password"
        {...register('password', { required: true })}
      />
      <Button type="submit" variant="filled" className="w-full" loading={loading}>
        Sign In
      </Button>
    </form>
  )
}
