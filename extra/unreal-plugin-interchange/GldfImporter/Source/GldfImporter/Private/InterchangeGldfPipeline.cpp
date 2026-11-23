#include "InterchangeGldfPipeline.h"

#include "GldfLuminaireActor.h"
#include "Nodes/InterchangeBaseNodeContainer.h"
#include "InterchangeActorFactoryNode.h"
#include "Logging/LogMacros.h"

#include UE_INLINE_GENERATED_CPP_BY_NAME(InterchangeGldfPipeline)

DEFINE_LOG_CATEGORY_STATIC(LogGldfPipeline, Log, All);

void UInterchangeGldfPipeline::ExecutePipeline(
    UInterchangeBaseNodeContainer* BaseNodeContainer,
    const TArray<UInterchangeSourceData*>& /*SourceDatas*/,
    const FString& /*ContentBasePath*/)
{
    if (!BaseNodeContainer)
    {
        return;
    }

    // The generic level pipeline (which ran before us in the scene stack)
    // has already created the actor factory node for the single scene node
    // our translator emitted — the luminaire root, which references the body
    // mesh. Our translator names the root scene node UID ".../root" and the
    // factory UID is that prefixed with "Factory_".
    //
    // Retype that actor factory node to AGldfLuminaireActor so the placed
    // luminaire is our custom actor. Because its object class is now
    // AGldfLuminaireActor, the Interchange manager dispatches it to
    // UInterchangeGldfActorFactory (most-derived IsChildOf wins over the stock
    // AStaticMeshActor factory). That factory fills the actor's BodyMesh +
    // emitter light + variant table directly, so the import is ONE
    // self-contained, draggable luminaire.
    //
    // Resolvable class path for FindObject<UClass> used by
    // UInterchangeActorFactoryNode::GetObjectClass(). Derived from the class,
    // never hard-coded: when the actor moved from the GldfImporter module to
    // GldfRuntime a stale literal made every import silently fall back to a
    // plain AActor (no lights, no variants).
    const FString LuminaireClassPath = AGldfLuminaireActor::StaticClass()->GetPathName();

    int32 Retyped = 0;
    BaseNodeContainer->IterateNodesOfType<UInterchangeActorFactoryNode>(
        [&](const FString& Uid, UInterchangeActorFactoryNode* ActorNode)
        {
            // Match the root: our translator's root scene node UID ends in
            // "\root" and lives under "\Scene\Gldf\". The factory UID keeps
            // that suffix (only a "Factory_" prefix is added).
            if (!Uid.Contains(TEXT("\\Scene\\Gldf\\")) || !Uid.EndsWith(TEXT("\\root")))
            {
                return;
            }
            ActorNode->SetCustomActorClassName(LuminaireClassPath);
            ++Retyped;
            UE_LOG(LogGldfPipeline, Log,
                TEXT("ExecutePipeline: retyped root actor %s → AGldfLuminaireActor"), *Uid);
        });

    if (Retyped == 0)
    {
        UE_LOG(LogGldfPipeline, Warning,
            TEXT("ExecutePipeline: no GLDF root actor factory node found to retype "
                 "(luminaire will import as a plain actor hierarchy)"));
    }
}
