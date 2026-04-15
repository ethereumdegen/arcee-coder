import Badge from '../ui/Badge'

interface StatusBadgeProps {
  status: string
  onChange?: (status: string) => void
  editable?: boolean
}

const statuses = ['new', 'reviewed', 'contacted', 'closed']

export default function StatusBadge({ status, onChange, editable = false }: StatusBadgeProps) {
  if (!editable) return <Badge status={status} />

  return (
    <select
      value={status}
      onChange={(e) => onChange?.(e.target.value)}
      className="px-3 py-1 rounded-full text-xs font-medium border-none bg-gray-100 cursor-pointer
        focus:outline-none focus:ring-2 focus:ring-accent/50"
    >
      {statuses.map((s) => (
        <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>
      ))}
    </select>
  )
}
