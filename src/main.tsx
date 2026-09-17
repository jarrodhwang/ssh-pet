import { createRoot } from 'react-dom/client';
import { App } from './App';
import { Pet } from './Pet';
import './style.css';

const petWindow = new URLSearchParams(location.search).get('window') === 'pet';
createRoot(document.getElementById('app')!).render(petWindow ? <Pet /> : <App />);
