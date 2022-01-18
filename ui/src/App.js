import RepositoryList, { useRepositoryList } from './components/repository/RepositoryList';
import './App.css';

function App() {
  const {repos, setRepoProperty, removeRepository, addNewRepository} = useRepositoryList()
  return (
    <RepositoryList repos={repos} setRepoProperty={setRepoProperty} removeRepository={removeRepository} addNewRepository={addNewRepository}/>
  )
}

export default App;
