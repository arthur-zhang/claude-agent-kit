import { useState, type KeyboardEvent, useRef, useEffect } from 'react';
import { Textarea } from '../ui/textarea';
import { Button } from '../ui/button';
import { Sparkles, Brain, Map, Plus, ArrowUp } from 'lucide-react';

interface MessageInputProps {
  onSend: (message: string) => void;
  onInterrupt?: () => void;
  disabled?: boolean;
  isProcessing?: boolean;
  placeholder?: string;
}

export function MessageInput({
  onSend,
  onInterrupt,
  disabled = false,
  isProcessing = false,
  placeholder = 'Ask to make changes, @mention files, run /commands',
}: MessageInputProps) {
  const [input, setInput] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const isComposingRef = useRef(false);

  const handleSend = () => {
    if (input.trim() && !disabled && !isProcessing) {
      onSend(input.trim());
      setInput('');
      // Reset height
      if (textareaRef.current) {
        textareaRef.current.style.height = 'auto';
        // Force focus back to textarea
        textareaRef.current.focus();
      }
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Send on Enter (without Shift) and not composing
    if (e.key === 'Enter' && !e.shiftKey && !isComposingRef.current) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleCompositionStart = () => {
    isComposingRef.current = true;
  };

  const handleCompositionEnd = () => {
    isComposingRef.current = false;
  };

  // Auto-resize
  useEffect(() => {
    if (textareaRef.current) {
        // Reset height to auto to get the correct scrollHeight for shrinking
        textareaRef.current.style.height = 'auto';
        // Set new height
        textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 200)}px`;
    }
  }, [input]);

  return (
    <div className="p-4 bg-background">
      <div className="w-full">
        {/* Main input container with theme-aware styling */}
        <div className="relative bg-card border border-border rounded-2xl shadow-lg overflow-hidden">
          {/* Keyboard shortcut hint */}
          <div className="absolute top-4 right-4 text-xs text-muted-foreground pointer-events-none">
            ⌘L to focus
          </div>

          {/* Textarea */}
          <Textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            placeholder={placeholder}
            disabled={disabled}
            className="min-h-[120px] max-h-[300px] resize-none px-4 pt-12 pb-16 bg-transparent border-0 text-foreground placeholder:text-muted-foreground focus-visible:ring-0 focus-visible:ring-offset-0 text-base"
            rows={1}
          />

          {/* Bottom toolbar */}
          <div className="absolute bottom-0 left-0 right-0 flex items-center justify-between px-4 py-3 border-t border-border">
            {/* Left side - Model selector and tools */}
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                className="h-8 px-3 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
              >
                <Sparkles className="h-4 w-4 mr-2" />
                <span className="text-sm">Sonnet 4.5</span>
              </Button>

              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
              >
                <Brain className="h-4 w-4" />
              </Button>

              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
              >
                <Map className="h-4 w-4" />
              </Button>
            </div>

            {/* Right side - Action buttons */}
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
              >
                <Plus className="h-4 w-4" />
              </Button>

              {isProcessing ? (
                // Interrupt button - circular stop button
                <Button
                  onClick={onInterrupt}
                  size="icon"
                  className="h-8 w-8 rounded-full bg-muted hover:bg-muted/80 text-foreground border border-border transition-all"
                >
                  <div className="h-3 w-3 rounded-sm bg-foreground" />
                </Button>
              ) : (
                // Send button when not processing
                <Button
                  onClick={handleSend}
                  disabled={disabled || !input.trim()}
                  size="icon"
                  className="h-8 w-8 bg-primary/10 hover:bg-primary/20 text-foreground disabled:bg-muted disabled:text-muted-foreground rounded-lg transition-all"
                >
                  <ArrowUp className="h-4 w-4" />
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}