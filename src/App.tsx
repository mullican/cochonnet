import { Routes, Route } from 'react-router-dom';
import { Layout } from './components/Layout';
import { TournamentList } from './features/tournaments/TournamentList';
import { TournamentDetail } from './features/tournaments/TournamentDetail';
import { TournamentCreate } from './features/tournaments/TournamentCreate';
import { TournamentEdit } from './features/tournaments/TournamentEdit';

function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<TournamentList />} />
        <Route path="tournaments/new" element={<TournamentCreate />} />
        <Route path="tournaments/:id" element={<TournamentDetail />} />
        <Route path="tournaments/:id/edit" element={<TournamentEdit />} />
      </Route>
    </Routes>
  );
}

export default App;
