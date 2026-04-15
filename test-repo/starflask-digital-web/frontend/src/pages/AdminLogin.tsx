import { useNavigate } from 'react-router-dom'
import { Helmet } from 'react-helmet-async'
import Container from '../components/ui/Container'
import LoginForm from '../components/admin/LoginForm'

export default function AdminLogin() {
  const navigate = useNavigate()

  return (
    <>
      <Helmet>
        <title>Admin Login — Starflask Digital</title>
      </Helmet>
      <div className="min-h-screen pt-24 pb-16 bg-gray-50 flex items-center">
        <Container>
          <div className="text-center mb-10">
            <h1 className="text-3xl font-heading font-bold text-gray-900">Admin Login</h1>
            <p className="mt-2 text-gray-500">Enter your password to access the dashboard.</p>
          </div>
          <LoginForm onSuccess={() => navigate('/admin')} />
        </Container>
      </div>
    </>
  )
}
