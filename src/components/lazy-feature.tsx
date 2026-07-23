import { lazy, Suspense, useState, type ComponentType } from "react";
import { AlertTriangle, LoaderCircle, RotateCw } from "lucide-react";
import { ErrorBoundary } from "react-error-boundary";
import { useTranslation } from "react-i18next";
import { Button } from "./ui/button";
import { cn } from "../lib/utils";

export type LazyFeatureLoader<TProps extends object> = () => Promise<{
  default: ComponentType<TProps>;
}>;

export function LazyFeature<TProps extends object>({
  className,
  componentProps,
  loader,
}: {
  className?: string;
  componentProps: TProps;
  loader: LazyFeatureLoader<TProps>;
}) {
  const { t } = useTranslation();
  const [LazyComponent, setLazyComponent] = useState(() => lazy(loader));

  return (
    <ErrorBoundary
      fallbackRender={({ resetErrorBoundary }) => (
        <div className={cn("flex min-h-40 flex-col items-center justify-center gap-3 p-6 text-center", className)} role="alert">
          <AlertTriangle className="h-6 w-6 text-destructive" aria-hidden="true" />
          <p className="text-sm text-muted-foreground">{t("featureLoad.error")}</p>
          <Button onClick={resetErrorBoundary} size="sm" variant="outline">
            <RotateCw className="h-4 w-4" aria-hidden="true" />
            {t("featureLoad.retry")}
          </Button>
        </div>
      )}
      onReset={() => setLazyComponent(lazy(loader))}
    >
      <Suspense
        fallback={
          <div className={cn("flex min-h-40 items-center justify-center gap-2 p-6 text-sm text-muted-foreground", className)} role="status">
            <LoaderCircle className="h-5 w-5 animate-spin" aria-hidden="true" />
            {t("featureLoad.loading")}
          </div>
        }
      >
        <LazyComponent {...componentProps} />
      </Suspense>
    </ErrorBoundary>
  );
}
