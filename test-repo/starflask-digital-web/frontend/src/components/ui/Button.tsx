import { type ButtonHTMLAttributes } from 'react'

type Variant = 'filled' | 'outlined' | 'dark'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
  size?: 'sm' | 'md' | 'lg'
  loading?: boolean
}

const variantClasses: Record<Variant, string> = {
  filled:
    'bg-accent text-white hover:bg-accent-dark shadow-[0_0_20px_rgba(167,139,250,0.3)] hover:shadow-[0_0_30px_rgba(167,139,250,0.5)]',
  outlined:
    'border-2 border-accent/60 text-white hover:bg-accent/10 hover:border-accent',
  dark:
    'bg-white/10 text-white border border-gray-700 hover:bg-white/20 hover:border-gray-500',
}

const sizeClasses = {
  sm: 'px-6 py-2.5 text-sm',
  md: 'px-8 py-3.5 text-base',
  lg: 'px-10 py-4 text-lg',
}

export default function Button({
  variant = 'filled',
  size = 'md',
  loading,
  children,
  disabled,
  className = '',
  ...props
}: ButtonProps) {
  return (
    <button
      className={`rounded-full font-heading font-semibold transition-all duration-300 cursor-pointer
        disabled:opacity-50 disabled:cursor-not-allowed
        ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? (
        <span className="flex items-center gap-2">
          <span className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
          Loading...
        </span>
      ) : (
        children
      )}
    </button>
  )
}
