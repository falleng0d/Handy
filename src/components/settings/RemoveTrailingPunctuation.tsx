import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface RemoveTrailingPunctuationProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const RemoveTrailingPunctuation: React.FC<RemoveTrailingPunctuationProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const removeTrailingPunctuationEnabled =
      getSetting("remove_trailing_punctuation") || false;

    return (
      <ToggleSwitch
        checked={removeTrailingPunctuationEnabled}
        onChange={(enabled) =>
          updateSetting("remove_trailing_punctuation", enabled)
        }
        isUpdating={isUpdating("remove_trailing_punctuation")}
        label={t("settings.debug.removeTrailingPunctuation.label")}
        description={t("settings.debug.removeTrailingPunctuation.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
