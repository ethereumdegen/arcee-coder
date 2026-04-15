import { useState, useEffect, useCallback } from 'react'
import { api } from '../../lib/api'
import { PROJECT_TYPE_LABELS } from '../../lib/constants'
import Badge from '../ui/Badge'
import SubmissionDetail from './SubmissionDetail'

export default function SubmissionsTable() {
  const [submissions, setSubmissions] = useState<Record<string, unknown>[]>([])
  const [loading, setLoading] = useState(true)
  const [statusFilter, setStatusFilter] = useState('all')
  const [typeFilter, setTypeFilter] = useState('all')
  const [selected, setSelected] = useState<Record<string, unknown> | null>(null)

  const fetchSubmissions = useCallback(async () => {
    try {
      const params: Record<string, string> = {}
      if (statusFilter !== 'all') params.status = statusFilter
      if (typeFilter !== 'all') params.type = typeFilter
      const data = await api.getSubmissions(params)
      setSubmissions(data.submissions as Record<string, unknown>[])
    } catch (err) {
      console.error('Failed to fetch submissions:', err)
    } finally {
      setLoading(false)
    }
  }, [statusFilter, typeFilter])

  useEffect(() => {
    fetchSubmissions()
  }, [fetchSubmissions])

  const openDetail = async (id: string) => {
    try {
      const data = await api.getSubmission(id)
      setSelected(data.submission as Record<string, unknown>)
    } catch (err) {
      console.error('Failed to fetch detail:', err)
    }
  }

  return (
    <div>
      <div className="flex flex-wrap gap-4 mb-6">
        <select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
          className="px-4 py-2 rounded-xl border border-gray-200 text-sm bg-white"
        >
          <option value="all">All Statuses</option>
          <option value="new">New</option>
          <option value="reviewed">Reviewed</option>
          <option value="contacted">Contacted</option>
          <option value="closed">Closed</option>
        </select>

        <select
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
          className="px-4 py-2 rounded-xl border border-gray-200 text-sm bg-white"
        >
          <option value="all">All Types</option>
          <option value="inbound_sales">Inbound Sales</option>
          <option value="web_design">Web Design</option>
          <option value="agentic_automation">Agentic Automation</option>
          <option value="multiple">Multiple</option>
        </select>
      </div>

      {loading ? (
        <div className="text-center py-12 text-gray-400">Loading submissions...</div>
      ) : submissions.length === 0 ? (
        <div className="text-center py-12 text-gray-400">No submissions yet.</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left">
            <thead>
              <tr className="border-b border-gray-200">
                <th className="pb-3 text-xs text-gray-400 uppercase font-medium">Date</th>
                <th className="pb-3 text-xs text-gray-400 uppercase font-medium">Name</th>
                <th className="pb-3 text-xs text-gray-400 uppercase font-medium">Email</th>
                <th className="pb-3 text-xs text-gray-400 uppercase font-medium">Type</th>
                <th className="pb-3 text-xs text-gray-400 uppercase font-medium">Budget</th>
                <th className="pb-3 text-xs text-gray-400 uppercase font-medium">Status</th>
              </tr>
            </thead>
            <tbody>
              {submissions.map((s) => (
                <tr
                  key={s.id as string}
                  onClick={() => openDetail(s.id as string)}
                  className="border-b border-gray-100 hover:bg-gray-50 cursor-pointer transition-colors"
                >
                  <td className="py-4 text-sm text-gray-500">
                    {new Date(s.created_at as string).toLocaleDateString()}
                  </td>
                  <td className="py-4 text-sm font-medium text-gray-800">{s.full_name as string}</td>
                  <td className="py-4 text-sm text-gray-500">{s.email as string}</td>
                  <td className="py-4 text-sm text-gray-500">
                    {PROJECT_TYPE_LABELS[s.project_type as string] || String(s.project_type)}
                  </td>
                  <td className="py-4 text-sm text-gray-500">{String(s.budget_range)}</td>
                  <td className="py-4">
                    <Badge status={s.status as string} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {selected && (
        <SubmissionDetail
          submission={selected}
          onUpdate={() => {
            fetchSubmissions()
            setSelected(null)
          }}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  )
}
