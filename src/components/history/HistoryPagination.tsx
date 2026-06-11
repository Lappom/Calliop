import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "../ui/Button";

interface HistoryPaginationProps {
  page: number;
  pageSize: number;
  total: number;
  disabled?: boolean;
  onPageChange: (page: number) => void;
}

export function HistoryPagination({
  page,
  pageSize,
  total,
  disabled = false,
  onPageChange,
}: HistoryPaginationProps) {
  if (total <= pageSize) {
    return null;
  }

  const totalPages = Math.ceil(total / pageSize);
  const canGoPrevious = page > 0;
  const canGoNext = page < totalPages - 1;
  const rangeStart = page * pageSize + 1;
  const rangeEnd = Math.min((page + 1) * pageSize, total);

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-t border-hairline pt-4">
      <p className="text-caption m-0 text-ash">
        {rangeStart}–{rangeEnd} sur {total.toLocaleString("fr-FR")}
      </p>

      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          disabled={disabled || !canGoPrevious}
          className="gap-1.5 px-3"
          onClick={() => onPageChange(page - 1)}
        >
          <ChevronLeft size={16} strokeWidth={1.75} />
          Précédent
        </Button>
        <span className="text-caption min-w-[5.5rem] text-center text-charcoal">
          Page {page + 1} / {totalPages}
        </span>
        <Button
          variant="ghost"
          disabled={disabled || !canGoNext}
          className="gap-1.5 px-3"
          onClick={() => onPageChange(page + 1)}
        >
          Suivant
          <ChevronRight size={16} strokeWidth={1.75} />
        </Button>
      </div>
    </div>
  );
}
