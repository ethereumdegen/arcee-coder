export default function FloatingOrbs({ className = '' }: { className?: string }) {
  return (
    <div className={`absolute inset-0 overflow-hidden pointer-events-none ${className}`}>
      <div
        className="absolute w-72 h-72 rounded-full bg-accent/20 blur-3xl animate-float"
        style={{ top: '10%', right: '5%', animationDelay: '0s' }}
      />
      <div
        className="absolute w-96 h-96 rounded-full bg-accent-dark/10 blur-3xl animate-float"
        style={{ bottom: '10%', left: '-5%', animationDelay: '2s' }}
      />
      <div
        className="absolute w-48 h-48 rounded-full bg-accent-light/15 blur-2xl animate-float"
        style={{ top: '40%', right: '20%', animationDelay: '4s' }}
      />
    </div>
  )
}
