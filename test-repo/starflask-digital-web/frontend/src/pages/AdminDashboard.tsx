import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Helmet } from 'react-helmet-async'
import { api } from '../lib/api'
import Container from '../components/ui/Container'
import Button from '../components/ui/Button'
import SubmissionsTable from '../components/admin/SubmissionsTable'

export default function AdminDashboard() {
  const navigate = useNavigate()
  const [authed, setAuthed] = useState<boolean | null>(null)

  useEffect(() => {
    api.getSubmissions()
      .then(() => setAuthed(true))
      .catch(() => {
        setAuthed(false)
        navigate('/admin/login')
      })
  }, [navigate])

  const handleLogout = async () => {
    await api.adminLogout()
    navigate('/admin/login')
  }

  if (authed === null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-2 border-accent border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <>
      <Helmet>
        <title>Dashboard — Starflask Digital</title>
      </Helmet>
      <div className="min-h-screen pt-8 pb-16 bg-gray-50">
        <Container>
          <div className="flex items-center justify-between mb-8">
            <div>
              <h1 className="text-3xl font-heading font-bold text-gray-900">Submissions</h1>
              <p className="text-gray-500 mt-1">Manage incoming project inquiries.</p>
            </div>
            <Button variant="dark" size="sm" onClick={handleLogout}>
              Logout
            </Button>
          </div>
          <SubmissionsTable />
        </Container>
      </div>
    </>
  )
}
