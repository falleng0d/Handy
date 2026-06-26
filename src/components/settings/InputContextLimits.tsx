import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { useOsType } from "../../hooks/useOsType";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";

type LimitKey =
  | "post_process_input_max_paragraphs"
  | "post_process_input_max_lines"
  | "post_process_input_max_chars";

export const InputContextLimits: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const osType = useOsType();

  // `${input}` is macOS-only, so these limits only apply there.
  if (osType !== "macos") return null;

  const handleChange =
    (key: LimitKey) => (event: React.ChangeEvent<HTMLInputElement>) => {
      const value = parseInt(event.target.value, 10);
      if (!isNaN(value) && value >= 0) {
        updateSetting(key, value);
      }
    };

  const renderField = (
    key: LimitKey,
    titleKey: string,
    descriptionKey: string,
  ) => (
    <SettingContainer
      title={t(titleKey)}
      description={t(descriptionKey)}
      descriptionMode="tooltip"
      grouped={true}
      layout="horizontal"
    >
      <div className="flex items-center space-x-2">
        <Input
          type="number"
          min="0"
          value={getSetting(key) ?? 0}
          onChange={handleChange(key)}
          disabled={isUpdating(key)}
          className="w-24"
        />
      </div>
    </SettingContainer>
  );

  return (
    <SettingsGroup
      title={t("settings.postProcessing.inputContextLimits.title")}
      description={t("settings.postProcessing.inputContextLimits.description")}
    >
      {renderField(
        "post_process_input_max_paragraphs",
        "settings.postProcessing.inputContextLimits.maxParagraphs.label",
        "settings.postProcessing.inputContextLimits.maxParagraphs.description",
      )}
      {renderField(
        "post_process_input_max_lines",
        "settings.postProcessing.inputContextLimits.maxLines.label",
        "settings.postProcessing.inputContextLimits.maxLines.description",
      )}
      {renderField(
        "post_process_input_max_chars",
        "settings.postProcessing.inputContextLimits.maxChars.label",
        "settings.postProcessing.inputContextLimits.maxChars.description",
      )}
    </SettingsGroup>
  );
};
