export default function GlowLine({ className = '' }: { className?: string }) {
  return (
    <div className={`relative h-px w-full overflow-hidden ${className}`}>
      <div className="absolute inset-0 bg-gradient-to-r from-transparent via-accent to-transparent animate-pulse-glow" />
    </div>
  )
}
