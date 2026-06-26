import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { useOsType } from "../../hooks/useOsType";

interface ContinuationLowercaseProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const ContinuationLowercase: React.FC<ContinuationLowercaseProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const osType = useOsType();

    if (osType !== "macos") return null;

    const continuationLowercaseEnabled =
      getSetting("continuation_lowercase") || false;

    return (
      <ToggleSwitch
        checked={continuationLowercaseEnabled}
        onChange={(enabled) => updateSetting("continuation_lowercase", enabled)}
        isUpdating={isUpdating("continuation_lowercase")}
        label={t("settings.advanced.continuationLowercase.label")}
        description={t("settings.advanced.continuationLowercase.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
