import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { RadioGroup, RadioGroupItem } from '../ui/radio-group';
import { Checkbox } from '../ui/checkbox';
import { Label } from '../ui/label';
import { Separator } from '../ui/separator';
import { ChevronLeft, ChevronRight, Check } from 'lucide-react';
import type { AskUserQuestionRequest, QuestionAnswer } from '../../types';

interface UserQuestionDialogProps {
  request: AskUserQuestionRequest;
  onSubmit: (answers: QuestionAnswer[]) => void;
  onCancel: () => void;
}

export function UserQuestionDialog({
  request,
  onSubmit,
  onCancel,
}: UserQuestionDialogProps) {
  const [currentQuestionIndex, setCurrentQuestionIndex] = useState(0);
  const [selectedOptions, setSelectedOptions] = useState<Map<number, Set<string>>>(
    new Map()
  );

  const currentQuestion = request.questions[currentQuestionIndex];
  const isLastQuestion = currentQuestionIndex === request.questions.length - 1;
  const isFirstQuestion = currentQuestionIndex === 0;

  // Get selected options for current question
  const getCurrentSelections = (): Set<string> => {
    return selectedOptions.get(currentQuestionIndex) || new Set();
  };

  // Toggle option selection
  const toggleOption = (label: string) => {
    const current = getCurrentSelections();
    const newSelections = new Set(current);

    if (currentQuestion.multiSelect) {
      if (newSelections.has(label)) {
        newSelections.delete(label);
      } else {
        newSelections.add(label);
      }
    } else {
      // Single select - replace selection
      newSelections.clear();
      newSelections.add(label);
    }

    setSelectedOptions(new Map(selectedOptions).set(currentQuestionIndex, newSelections));
  };

  // Handle next question or submit
  const handleNext = () => {
    if (isLastQuestion) {
      // Build answers array
      const answers: QuestionAnswer[] = [];
      selectedOptions.forEach((selections, questionIndex) => {
        answers.push({
          question_index: questionIndex,
          selected: Array.from(selections),
        });
      });
      onSubmit(answers);
    } else {
      setCurrentQuestionIndex(currentQuestionIndex + 1);
    }
  };

  // Handle previous question
  const handlePrev = () => {
    if (!isFirstQuestion) {
      setCurrentQuestionIndex(currentQuestionIndex - 1);
    }
  };

  // Check if can proceed
  const canProceed = getCurrentSelections().size > 0;

  return (
    <Dialog open={true}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            ❓ Claude has a question
          </DialogTitle>
          <DialogDescription>
            Please select your preference to continue
          </DialogDescription>
        </DialogHeader>

        {/* Progress indicator for multiple questions */}
        {request.questions.length > 1 && (
          <div className="space-y-2">
            <div className="flex gap-2">
              {request.questions.map((q, idx) => (
                <button
                  key={idx}
                  onClick={() => setCurrentQuestionIndex(idx)}
                  className={`flex-1 h-2 rounded-full transition-all ${
                    idx === currentQuestionIndex
                      ? 'bg-primary'
                      : idx < currentQuestionIndex
                      ? 'bg-green-500'
                      : 'bg-muted'
                  }`}
                  title={q.header || `Question ${idx + 1}`}
                />
              ))}
            </div>
            <div className="flex justify-between text-xs text-muted-foreground">
              {request.questions.map((q, idx) => (
                <span
                  key={idx}
                  className={
                    idx === currentQuestionIndex ? 'text-primary font-semibold' : ''
                  }
                >
                  {q.header || `Q${idx + 1}`}
                </span>
              ))}
            </div>
          </div>
        )}

        <Separator />

        {/* Question content */}
        <div className="space-y-4">
          {/* Question header */}
          {currentQuestion.header && (
            <Badge variant="secondary">{currentQuestion.header}</Badge>
          )}

          {/* Question text */}
          <p className="text-lg font-medium whitespace-pre-wrap">
            {currentQuestion.question}
          </p>

          {/* Selection mode hint */}
          <Badge variant={currentQuestion.multiSelect ? 'default' : 'outline'}>
            {currentQuestion.multiSelect ? '☑️ Multiple Selection' : '⚪ Single Selection'}
          </Badge>

          {/* Options */}
          <div className="space-y-3">
            {currentQuestion.multiSelect ? (
              // Multi-select with checkboxes
              currentQuestion.options.map((option) => {
                const isSelected = getCurrentSelections().has(option.label);
                return (
                  <div
                    key={option.label}
                    className={`flex items-start space-x-3 p-4 rounded-lg border-2 transition-all cursor-pointer ${
                      isSelected
                        ? 'border-primary bg-primary/5'
                        : 'border-border hover:border-primary/50'
                    }`}
                    onClick={() => toggleOption(option.label)}
                  >
                    <Checkbox
                      checked={isSelected}
                      onCheckedChange={() => toggleOption(option.label)}
                      id={option.label}
                    />
                    <div className="flex-1">
                      <Label
                        htmlFor={option.label}
                        className="font-semibold cursor-pointer"
                      >
                        {option.label}
                      </Label>
                      {option.description && (
                        <p className="text-sm text-muted-foreground mt-1">
                          {option.description}
                        </p>
                      )}
                    </div>
                    {isSelected && (
                      <Badge variant="default" className="shrink-0">
                        Selected
                      </Badge>
                    )}
                  </div>
                );
              })
            ) : (
              // Single-select with radio buttons
              <RadioGroup
                value={Array.from(getCurrentSelections())[0] || ''}
                onValueChange={(value) => toggleOption(value)}
              >
                {currentQuestion.options.map((option) => {
                  const isSelected = getCurrentSelections().has(option.label);
                  return (
                    <div
                      key={option.label}
                      className={`flex items-start space-x-3 p-4 rounded-lg border-2 transition-all cursor-pointer ${
                        isSelected
                          ? 'border-primary bg-primary/5'
                          : 'border-border hover:border-primary/50'
                      }`}
                      onClick={() => toggleOption(option.label)}
                    >
                      <RadioGroupItem value={option.label} id={option.label} />
                      <div className="flex-1">
                        <Label
                          htmlFor={option.label}
                          className="font-semibold cursor-pointer"
                        >
                          {option.label}
                        </Label>
                        {option.description && (
                          <p className="text-sm text-muted-foreground mt-1">
                            {option.description}
                          </p>
                        )}
                      </div>
                      {isSelected && (
                        <Badge variant="default" className="shrink-0">
                          Selected
                        </Badge>
                      )}
                    </div>
                  );
                })}
              </RadioGroup>
            )}
          </div>
        </div>

        <DialogFooter className="flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            {request.questions.length > 1 && (
              <span>
                Question {currentQuestionIndex + 1} of {request.questions.length}
              </span>
            )}
          </div>

          <div className="flex gap-2">
            <Button variant="outline" onClick={onCancel}>
              Cancel
            </Button>

            {!isFirstQuestion && (
              <Button variant="secondary" onClick={handlePrev}>
                <ChevronLeft className="h-4 w-4 mr-1" />
                Back
              </Button>
            )}

            <Button onClick={handleNext} disabled={!canProceed}>
              {isLastQuestion ? (
                <>
                  Submit
                  <Check className="h-4 w-4 ml-1" />
                </>
              ) : (
                <>
                  Next
                  <ChevronRight className="h-4 w-4 ml-1" />
                </>
              )}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
