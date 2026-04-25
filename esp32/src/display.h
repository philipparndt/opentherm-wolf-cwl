#pragma once

#include <Arduino.h>

// Display page indices
#define PAGE_HOME        0
#define PAGE_TEMPERATURE 1
#define PAGE_STATUS      2
#define PAGE_SYSTEM      3
#define PAGE_COUNT       4

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
