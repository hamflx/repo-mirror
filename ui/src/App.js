import {useState, useEffect} from 'react'

import logo from './logo.svg';
import './App.css';

const useAsyncEffect = (fn, ...args) => {
  useEffect(() => {
    fn()
  }, ...args)
}

function App() {
  const [repos, setRepos] = useState([])
  useAsyncEffect(async () => {
    setRepos(await (await fetch('/api/repos')).json())
  }, [])
  const items = repos.map(repo => {
    return (
      <div style={{display: 'flex', padding: '20px 0', borderBottom: '1px solid #ccc'}} key={repo.source}>
        <div style={{width: '500px'}}>{repo.source}</div>
        <div style={{width: '500px'}}>{repo.mirror}</div>
      </div>
    )
  })
  return (
    <div>
      {items}
    </div>
  )
}

export default App;
