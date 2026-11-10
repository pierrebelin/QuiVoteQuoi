import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter, Navigate, Routes, Route } from 'react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import './index.css'
import App from './App.tsx'
import LandingPage from './pages/LandingPage.tsx'
import ThemeGroupEntryPage from './pages/ThemeGroupEntryPage.tsx'
import DossiersRedirect from './pages/DossiersRedirect.tsx'
import DossierDetailPage from './pages/DossierDetailPage.tsx'
import GroupListPage from './pages/GroupListPage.tsx'
import GroupDetailPage from './pages/GroupDetailPage.tsx'
import GroupVotesPage from './pages/GroupVotesPage.tsx'
import FinalVotesMethodPage from './pages/FinalVotesMethodPage.tsx'
import ScrutinListPage from './pages/ScrutinListPage.tsx'
import ScrutinDetailPage from './pages/ScrutinDetailPage.tsx'
import ThemeMethodPage from './pages/ThemeMethodPage.tsx'
import ThemeArbitrationPage from './pages/ThemeArbitrationPage.tsx'
import UnassignedTextsPage from './pages/UnassignedTextsPage.tsx'
import TextDetailPage from './pages/TextDetailPage.tsx'
import ComprendrePage from './pages/ComprendrePage.tsx'
import CandidateComparisonPage from './pages/CandidateComparisonPage.tsx'
import ThemeRedirect from './pages/ThemeRedirect.tsx'
import { ENTRY_PATH } from './lib/entryFlow'

const queryClient = new QueryClient()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<App />}>
            <Route index element={<LandingPage />} />
            <Route path="par-sujet" element={<ThemeGroupEntryPage />} />
            <Route path="a-propos" element={<Navigate to="/" replace />} />
            <Route path="dossiers" element={<DossiersRedirect />} />
            <Route path="dossiers/:uid" element={<DossierDetailPage />} />
            <Route path="groupes" element={<GroupListPage />} />
            <Route path="groupes/:uid" element={<GroupDetailPage />} />
            <Route path="votes-par-groupe" element={<GroupVotesPage />} />
            <Route path="candidats-2027" element={<CandidateComparisonPage />} />
            <Route path="votes-par-groupe/methode" element={<FinalVotesMethodPage />} />
            <Route path="scrutins" element={<ScrutinListPage />} />
            <Route path="scrutins/:uid" element={<ScrutinDetailPage />} />
            <Route path="themes" element={<Navigate to={ENTRY_PATH} replace />} />
            <Route path="themes/methode" element={<ThemeMethodPage />} />
            <Route path="themes/non-rattaches" element={<UnassignedTextsPage />} />
            <Route path="themes/arbitrage" element={<ThemeArbitrationPage />} />
            <Route path="themes/:code" element={<ThemeRedirect />} />
            <Route path="textes/:key" element={<TextDetailPage />} />
            <Route path="comprendre" element={<ComprendrePage />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
)
