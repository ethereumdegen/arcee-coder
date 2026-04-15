const statusColors: Record<string, string> = {
  new: 'bg-blue-100 text-blue-700',
  reviewed: 'bg-yellow-100 text-yellow-700',
  contacted: 'bg-green-100 text-green-700',
  closed: 'bg-gray-100 text-gray-600',
}

interface BadgeProps {
  status: string
  className?: string
}

export default function Badge({ status, className = '' }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium capitalize
        ${statusColors[status] || 'bg-gray-100 text-gray-600'} ${className}`}
    >
      {status}
    </span>
  )
}
