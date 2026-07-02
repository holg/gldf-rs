// Scene factory that builds the single self-contained GLDF luminaire actor.
//
// Why a custom factory (Phase 3c): the stock UInterchangeStaticMeshActorFactory
// and UInterchangeLightActorFactory hard-Cast to AStaticMeshActor / ALight and
// fill GetStaticMeshComponent()/GetLightComponent(). They cannot populate the
// named components of our AGldfLuminaireActor. So we register our own factory
// whose GetFactoryClass() == AGldfLuminaireActor; the Interchange manager
// dispatches any actor factory node whose object class is AGldfLuminaireActor
// to us (most-derived IsChildOf match beats the generic AActor factory — see
// InterchangeManager.cpp GetRegisteredFactoryClass + InterchangeTaskParsing.cpp
// which selects the factory via FactoryNode->GetObjectClass()).
//
// Our pipeline (UInterchangeGldfPipeline) retypes the GLDF root actor factory
// node's ActorClassName to AGldfLuminaireActor so this dispatch fires.
//
// ProcessActor then, on the spawned AGldfLuminaireActor:
//   • assigns the imported UStaticMesh to BodyMesh (via the standard
//     FindAssetInstanceFactoryNode → GetCustomReferenceObject chain),
//   • creates the emitter light component (Rect or Point) from the L3D LEO
//     shape reported by the Rust FFI, placing + orienting it at the emitter,
//   • resolves the default variant's IES UTextureLightProfile and applies it,
//   • populates the actor's Variants[] (IES profile + lumens + CCT per variant)
//     so the Details-panel dropdown can hot-swap them.

#pragma once

#include "CoreMinimal.h"
#include "Scene/InterchangeActorFactory.h"
#include "InterchangeGldfActorFactory.generated.h"

class AGldfLuminaireActor;
class UInterchangeActorFactoryNode;
class UInterchangeBaseNodeContainer;

UCLASS(BlueprintType)
class GLDFIMPORTER_API UInterchangeGldfActorFactory : public UInterchangeActorFactory
{
    GENERATED_BODY()

public:
    /** Tells the Interchange manager this factory produces AGldfLuminaireActor.
     *  Registration (GldfImporterModule) maps this class → this factory; the
     *  parsing task then routes any actor factory node whose object class is
     *  AGldfLuminaireActor here. */
    virtual UClass* GetFactoryClass() const override;

protected:
    /** Fill the spawned AGldfLuminaireActor's components from the factory
     *  graph + the GLDF (via FFI). Returns the component the base class should
     *  apply node custom-attributes (transform/mobility) to — our BodyMesh
     *  root, matching how the stock factories return their root mesh/light
     *  component. */
    virtual UObject* ProcessActor(
        AActor& SpawnedActor,
        const UInterchangeActorFactoryNode& FactoryNode,
        const UInterchangeBaseNodeContainer& NodeContainer,
        const FImportSceneObjectsParams& Params) override;

private:
    /** Resolve the imported UStaticMesh referenced by this actor's target
     *  scene node and assign it to Luminaire.BodyMesh. */
    void ApplyBodyMesh(
        AGldfLuminaireActor& Luminaire,
        const UInterchangeActorFactoryNode& FactoryNode,
        const UInterchangeBaseNodeContainer& NodeContainer) const;

    /** Read the GLDF source path off the scene-import params, open the Rust
     *  variant table + LEO shape, build the light component, resolve IES
     *  profiles, and fill Luminaire.Variants. Applies the default variant. */
    void ApplyLightAndVariants(
        AGldfLuminaireActor& Luminaire,
        const UInterchangeActorFactoryNode& FactoryNode,
        const UInterchangeBaseNodeContainer& NodeContainer,
        const FImportSceneObjectsParams& Params) const;
};
