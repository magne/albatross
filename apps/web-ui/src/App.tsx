import { BrowserRouter, Link, Outlet, Route, Routes } from 'react-router'

// Placeholder components
function HomePage() {
  return <h1 className="text-2xl font-bold text-blue-600">Home Page</h1>
}

function AboutPage() {
  return <h1 className="text-2xl font-bold text-green-600">About Page</h1>
}

function Layout() {
  return (
    <div className="p-4">
      <nav className="mb-4">
        <ul className="flex space-x-4">
          <li>
            <Link to="/" className="text-blue-500 hover:underline">
              Home
            </Link>
          </li>
          <li>
            <Link to="/about" className="text-blue-500 hover:underline">
              About
            </Link>
          </li>
        </ul>
      </nav>
      <hr className="mb-4" />
      <Outlet /> {/* Child routes will render here */}
    </div>
  )
}

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<HomePage />} />
          <Route path="about" element={<AboutPage />} />
          {/* Add other routes here */}
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App
