import type { ProjectType } from '../../types'

const types: { value: ProjectType; label: string; desc: string; icon: string }[] = [
  { value: 'inbound_sales', label: 'Inbound Sales', desc: 'Funnels, lead capture, CRM', icon: '📈' },
  { value: 'web_design', label: 'Web Design', desc: 'Modern, high-converting sites', icon: '🎨' },
  { value: 'agentic_automation', label: 'Agentic Automation', desc: 'AI agents & workflows', icon: '🤖' },
  { value: 'multiple', label: 'Something Else', desc: 'Tell us what you need', icon: '⚡' },
]

interface StepProjectTypeProps {
  value: ProjectType | ''
  onChange: (val: ProjectType) => void
}

export default function StepProjectType({ value, onChange }: StepProjectTypeProps) {
  return (
    <div>
      <h3 className="text-2xl font-heading font-bold text-gray-900 mb-2">What do you need?</h3>
      <p className="text-gray-500 mb-8">Select the type of project you're looking for.</p>

      <div className="grid sm:grid-cols-2 gap-4">
        {types.map((t) => (
          <button
            key={t.value}
            type="button"
            onClick={() => onChange(t.value)}
            className={`p-6 rounded-2xl border-2 text-left transition-all duration-200 cursor-pointer
              ${value === t.value
                ? 'border-accent bg-accent/5 shadow-lg shadow-accent/10'
                : 'border-gray-200 hover:border-accent/30 bg-white'
              }`}
          >
            <span className="text-3xl mb-3 block">{t.icon}</span>
            <h4 className="font-heading font-semibold text-gray-900 mb-1">{t.label}</h4>
            <p className="text-sm text-gray-500">{t.desc}</p>
          </button>
        ))}
      </div>
    </div>
  )
}
