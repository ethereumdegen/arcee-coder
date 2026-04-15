import { useState } from 'react'
import { toast } from 'sonner'
import { api } from '../../lib/api'
import { PROJECT_TYPE_LABELS, BUDGET_LABELS, TIMELINE_LABELS } from '../../lib/constants'
import StatusBadge from './StatusBadge'
import Button from '../ui/Button'

interface SubmissionDetailProps {
  submission: Record<string, unknown>
  onUpdate: () => void
  onClose: () => void
}

export default function SubmissionDetail({ submission, onUpdate, onClose }: SubmissionDetailProps) {
  const [notes, setNotes] = useState((submission.notes as string) || '')
  const [saving, setSaving] = useState(false)

  const handleStatusChange = async (status: string) => {
    try {
      await api.updateSubmission(submission.id as string, { status })
      toast.success('Status updated')
      onUpdate()
    } catch {
      toast.error('Failed to update status')
    }
  }

  const handleSaveNotes = async () => {
    setSaving(true)
    try {
      await api.updateSubmission(submission.id as string, { notes })
      toast.success('Notes saved')
      onUpdate()
    } catch {
      toast.error('Failed to save notes')
    } finally {
      setSaving(false)
    }
  }

  const files = (submission.files as Array<Record<string, string>>) || []

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto p-8">
        <div className="flex items-start justify-between mb-6">
          <div>
            <h2 className="text-2xl font-heading font-bold text-gray-900">
              {submission.title as string}
            </h2>
            <p className="text-sm text-gray-500 mt-1">
              {new Date(submission.created_at as string).toLocaleDateString()}
            </p>
          </div>
          <button onClick={onClose} className="text-gray-400 hover:text-gray-600 cursor-pointer text-2xl">
            &times;
          </button>
        </div>

        <div className="grid grid-cols-2 gap-4 mb-6">
          <div>
            <label className="text-xs text-gray-400 uppercase">Type</label>
            <p className="text-gray-800">{PROJECT_TYPE_LABELS[submission.project_type as string]}</p>
          </div>
          <div>
            <label className="text-xs text-gray-400 uppercase">Status</label>
            <div className="mt-1">
              <StatusBadge
                status={submission.status as string}
                editable
                onChange={handleStatusChange}
              />
            </div>
          </div>
          <div>
            <label className="text-xs text-gray-400 uppercase">Budget</label>
            <p className="text-gray-800">{BUDGET_LABELS[submission.budget_range as string]}</p>
          </div>
          <div>
            <label className="text-xs text-gray-400 uppercase">Timeline</label>
            <p className="text-gray-800">{TIMELINE_LABELS[submission.timeline as string]}</p>
          </div>
        </div>

        <div className="mb-6">
          <label className="text-xs text-gray-400 uppercase">Description</label>
          <p className="text-gray-700 mt-1 leading-relaxed">{submission.description as string}</p>
        </div>

        <div className="grid grid-cols-2 gap-4 mb-6 p-4 bg-gray-50 rounded-xl">
          <div>
            <label className="text-xs text-gray-400 uppercase">Name</label>
            <p className="text-gray-800">{submission.full_name as string}</p>
          </div>
          <div>
            <label className="text-xs text-gray-400 uppercase">Email</label>
            <p className="text-gray-800">
              <a href={`mailto:${submission.email}`} className="text-accent hover:underline">
                {submission.email as string}
              </a>
            </p>
          </div>
          {submission.phone ? (
            <div>
              <label className="text-xs text-gray-400 uppercase">Phone</label>
              <p className="text-gray-800">{String(submission.phone)}</p>
            </div>
          ) : null}
          {submission.company ? (
            <div>
              <label className="text-xs text-gray-400 uppercase">Company</label>
              <p className="text-gray-800">{String(submission.company)}</p>
            </div>
          ) : null}
        </div>

        {files.length > 0 && (
          <div className="mb-6">
            <label className="text-xs text-gray-400 uppercase mb-2 block">Files</label>
            <div className="grid grid-cols-3 gap-3">
              {files.map((f) => (
                <a
                  key={f.storage_key}
                  href={f.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block rounded-xl overflow-hidden border border-gray-200 hover:border-accent transition-colors"
                >
                  {f.file_type?.startsWith('image/') ? (
                    <img src={f.url} alt={f.filename} className="w-full h-20 object-cover" />
                  ) : (
                    <div className="w-full h-20 bg-gray-50 flex items-center justify-center text-gray-400 text-xs">
                      PDF
                    </div>
                  )}
                  <p className="text-xs text-gray-500 p-2 truncate">{f.filename}</p>
                </a>
              ))}
            </div>
          </div>
        )}

        <div className="mb-6">
          <label className="text-xs text-gray-400 uppercase mb-2 block">Internal Notes</label>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className="w-full px-4 py-3 rounded-xl border border-gray-200 bg-white text-gray-800 resize-none
              focus:outline-none focus:ring-2 focus:ring-accent/50"
            rows={3}
            placeholder="Add internal notes..."
          />
          <Button
            type="button"
            variant="dark"
            size="sm"
            className="mt-2"
            onClick={handleSaveNotes}
            loading={saving}
          >
            Save Notes
          </Button>
        </div>
      </div>
    </div>
  )
}
