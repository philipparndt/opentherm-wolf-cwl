#pragma once

#include <Arduino.h>

// Display page indices
#define PAGE_HOME        0
#define PAGE_BYPASS      1
#define PAGE_TEMP_IN     2
#define PAGE_TEMP_OUT    3
#define PAGE_STATUS      4
#define PAGE_SYSTEM      5
#define PAGE_COUNT       6

// Display state
extern int currentPage;
extern bool editMode;

// Display functions
void setupDisplay();
void updateDisplay();
void setPage(int page);
void nextPage();
void prevPage();

// Edit mode
void enterEditMode();
void exitEditMode(bool apply);
void adjustEditValue(int delta);

// Standby — returns true if display was woken (input should be discarded)
bool displayWake();
