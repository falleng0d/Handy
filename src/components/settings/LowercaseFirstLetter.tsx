import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface LowercaseFirstLetterProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const LowercaseFirstLetter: React.FC<LowercaseFirstLetterProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const lowercaseFirstLetterEnabled =
      getSetting("lowercase_first_letter") || false;

    return (
      <ToggleSwitch
        checked={lowercaseFirstLetterEnabled}
        onChange={(enabled) => updateSetting("lowercase_first_letter", enabled)}
        isUpdating={isUpdating("lowercase_first_letter")}
        label={t("settings.advanced.lowercaseFirstLetter.label")}
        description={t("settings.advanced.lowercaseFirstLetter.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
