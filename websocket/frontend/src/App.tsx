import { ThemeProvider } from './components/theme-provider';
import { ChatInterface } from './components/chat/chat-interface';
import './App.css';

function App() {
  return (
    <ThemeProvider>
      <ChatInterface />
    </ThemeProvider>
  );
}

export default App;
