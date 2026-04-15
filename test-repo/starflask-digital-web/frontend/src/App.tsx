import { Routes, Route } from 'react-router-dom'
import { lazy, Suspense } from 'react'
import Header from './components/layout/Header'
import Footer from './components/layout/Footer'

const Landing = lazy(() => import('./pages/Landing'))
const Intake = lazy(() => import('./pages/Intake'))
const IntakeSuccess = lazy(() => import('./pages/IntakeSuccess'))
const AdminLogin = lazy(() => import('./pages/AdminLogin'))
const AdminDashboard = lazy(() => import('./pages/AdminDashboard'))
const BlogPost = lazy(() => import('./pages/BlogPost'))
const BusinessRedirect = lazy(() => import('./pages/BusinessRedirect'))

function LoadingFallback() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-sf-black">
      <div className="w-8 h-8 border-2 border-accent border-t-transparent rounded-full animate-spin" />
    </div>
  )
}

export default function App() {
  return (
    <>
      <Header />
      <Suspense fallback={<LoadingFallback />}>
        <Routes>
          <Route path="/" element={<Landing />} />
          <Route path="/business" element={<BusinessRedirect />} />
          <Route path="/blog/:slug" element={<BlogPost />} />
          <Route path="/intake" element={<Intake />} />
          <Route path="/intake/success" element={<IntakeSuccess />} />
          <Route path="/admin/login" element={<AdminLogin />} />
          <Route path="/admin" element={<AdminDashboard />} />
        </Routes>
      </Suspense>
      <Footer />
    </>
  )
}
