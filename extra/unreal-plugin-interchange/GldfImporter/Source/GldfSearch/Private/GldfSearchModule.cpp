#include "GldfSearchModule.h"
#include "Modules/ModuleManager.h"

// Runtime half of GLDF Search: the cabi query engine, downloads and the Slate
// panel. The editor tab (Window > Tools) and the Interchange placement live in
// GldfSearchEditor; a game binds UGldfSearchLibrary::PlaceHandler() itself.
void FGldfSearchModule::StartupModule() {}
void FGldfSearchModule::ShutdownModule() {}

IMPLEMENT_MODULE(FGldfSearchModule, GldfSearch)
