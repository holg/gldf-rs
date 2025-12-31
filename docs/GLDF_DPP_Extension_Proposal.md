# GLDF XSD Extension Proposal for EU Digital Product Passport (DPP)

**Document Version:** 1.0
**Date:** 2025-12-18
**Status:** DRAFT - Change Request Proposal
**Target GLDF Version:** 1.1.0 or 2.0.0

---

## 1. Executive Summary

The European Union's Digital Product Passport (DPP) regulation, part of the Ecodesign for Sustainable Products Regulation (ESPR), will require comprehensive product data for lighting products starting in 2026-2027. This document proposes extensions to the GLDF XSD schema to support DPP compliance.

### Key Dates
- **2026:** DPP requirements expected for lighting products
- **2027:** Full enforcement with EU registry integration

### Scope
This proposal adds **29 new elements** across 4 priority categories to support:
- Product identification and traceability
- Environmental impact (carbon footprint, EPD)
- Material composition and circularity
- Repairability and durability
- Compliance and hazardous substances

---

## 2. Current GLDF Structure Analysis

### Existing Elements (Already DPP-Ready)
| Element | Location | DPP Requirement |
|---------|----------|-----------------|
| GTIN | Variant, LightSource | Product identification |
| LightSource Lifetime | LightSource | Durability (partial) |
| EnergyLabels | LightSource | Energy efficiency |

### Integration Points for New Elements
| Location | New Elements Count | Rationale |
|----------|-------------------|-----------|
| **Variant** | 18 | Variant-specific sustainability data |
| **ProductMetaData** | 6 | Global product compliance data |
| **Header** | 2 | Manufacturer-level programs |
| **New: DigitalProductPassport** | 3 | DPP-specific identification |

---

## 3. Proposed XSD Extensions

### 3.1 New Complex Types

#### 3.1.1 DigitalProductPassport Type
```xml
<xs:complexType name="DigitalProductPassport">
    <xs:annotation>
        <xs:documentation>EU Digital Product Passport identification and data carrier information</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="DPPUniqueIdentifier" type="xs:string" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Unique identifier for EU DPP registry (required from 2026)</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="DataCarrierURI" type="xs:anyURI" minOccurs="0">
            <xs:annotation>
                <xs:documentation>URI for QR code/data carrier linking to DPP data</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="DataCarrierType" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Type of data carrier used on the product</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:string">
                    <xs:enumeration value="QRCode"/>
                    <xs:enumeration value="DataMatrix"/>
                    <xs:enumeration value="RFID"/>
                    <xs:enumeration value="NFC"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

#### 3.1.2 CarbonFootprint Type
```xml
<xs:complexType name="CarbonFootprint">
    <xs:annotation>
        <xs:documentation>Carbon footprint data according to ISO 14067 / PEF methodology</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="TotalCO2e" type="xs:double">
            <xs:annotation>
                <xs:documentation>Total carbon footprint in kg CO2 equivalent</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="CalculationMethod" minOccurs="0">
            <xs:simpleType>
                <xs:restriction base="xs:string">
                    <xs:enumeration value="ISO14067"/>
                    <xs:enumeration value="PEF"/>
                    <xs:enumeration value="GHGProtocol"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="ByPhase" minOccurs="0">
            <xs:complexType>
                <xs:sequence>
                    <xs:element name="RawMaterials" type="xs:double" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>kg CO2e from raw material extraction</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                    <xs:element name="Manufacturing" type="xs:double" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>kg CO2e from manufacturing process</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                    <xs:element name="Transport" type="xs:double" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>kg CO2e from transportation</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                    <xs:element name="Use" type="xs:double" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>kg CO2e from use phase (lifetime energy)</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                    <xs:element name="EndOfLife" type="xs:double" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>kg CO2e from end-of-life processing</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                </xs:sequence>
            </xs:complexType>
        </xs:element>
        <xs:element name="EPDReference" type="xs:anyURI" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Link to Environmental Product Declaration (EPD)</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="EPDNumber" type="xs:string" minOccurs="0">
            <xs:annotation>
                <xs:documentation>EPD registration number</xs:documentation>
            </xs:annotation>
        </xs:element>
    </xs:sequence>
    <xs:attribute name="functionalUnit" type="xs:string">
        <xs:annotation>
            <xs:documentation>Functional unit for calculation (e.g., "per luminaire", "per 1000 lm for 50000h")</xs:documentation>
        </xs:annotation>
    </xs:attribute>
</xs:complexType>
```

#### 3.1.3 MaterialComposition Type
```xml
<xs:complexType name="MaterialComposition">
    <xs:annotation>
        <xs:documentation>Material composition of the luminaire for circularity assessment</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="TotalWeight" type="xs:double">
            <xs:annotation>
                <xs:documentation>Total weight of the luminaire in grams</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="Materials" minOccurs="0">
            <xs:complexType>
                <xs:sequence>
                    <xs:element name="Material" maxOccurs="unbounded">
                        <xs:complexType>
                            <xs:sequence>
                                <xs:element name="Name" type="xs:string"/>
                                <xs:element name="Category">
                                    <xs:simpleType>
                                        <xs:restriction base="xs:string">
                                            <xs:enumeration value="Metal"/>
                                            <xs:enumeration value="Plastic"/>
                                            <xs:enumeration value="Glass"/>
                                            <xs:enumeration value="Electronics"/>
                                            <xs:enumeration value="Ceramic"/>
                                            <xs:enumeration value="Composite"/>
                                            <xs:enumeration value="Other"/>
                                        </xs:restriction>
                                    </xs:simpleType>
                                </xs:element>
                                <xs:element name="Weight" type="xs:double">
                                    <xs:annotation>
                                        <xs:documentation>Weight in grams</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="Percentage" type="xs:double" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>Percentage of total weight (0-100)</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="RecycledContent" type="xs:double" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>Percentage of recycled content in this material (0-100)</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="Recyclable" type="xs:boolean" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>Whether this material is recyclable</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                            </xs:sequence>
                        </xs:complexType>
                    </xs:element>
                </xs:sequence>
            </xs:complexType>
        </xs:element>
        <xs:element name="RecycledContentTotal" type="xs:double" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Total percentage of recycled materials (0-100)</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="RecyclabilityTotal" type="xs:double" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Total percentage of recyclable materials (0-100)</xs:documentation>
            </xs:annotation>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

#### 3.1.4 Circularity Type
```xml
<xs:complexType name="Circularity">
    <xs:annotation>
        <xs:documentation>End-of-life and circularity information</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="DisassemblyTime" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Time required to disassemble the luminaire in minutes</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:int">
                    <xs:minInclusive value="0"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="DisassemblyInstructions" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Reference to disassembly instructions file</xs:documentation>
            </xs:annotation>
            <xs:complexType>
                <xs:attribute name="fileId" type="xs:NCName" use="required"/>
            </xs:complexType>
        </xs:element>
        <xs:element name="RecyclingInstructions" type="Locale" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Per-component recycling guidance, translatable</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="WasteCode" type="xs:string" minOccurs="0">
            <xs:annotation>
                <xs:documentation>European Waste Catalogue (EWC) code</xs:documentation>
            </xs:annotation>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

#### 3.1.5 Durability Type
```xml
<xs:complexType name="Durability">
    <xs:annotation>
        <xs:documentation>Product lifespan and warranty information</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="ExpectedLifespanYears" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Expected lifespan of the luminaire in years (not just LED hours)</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:int">
                    <xs:minInclusive value="0"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="WarrantyYears" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Product warranty period in years</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:int">
                    <xs:minInclusive value="0"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="WarrantyTerms" type="Locale" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Warranty terms description, translatable</xs:documentation>
            </xs:annotation>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

#### 3.1.6 Repairability Type
```xml
<xs:complexType name="Repairability">
    <xs:annotation>
        <xs:documentation>Repairability and spare parts information per EU requirements</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="RepairabilityScore" minOccurs="0">
            <xs:annotation>
                <xs:documentation>EU Repairability Index score (1-10 scale)</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:double">
                    <xs:minInclusive value="1"/>
                    <xs:maxInclusive value="10"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="RepairInstructions" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Reference to repair instructions file</xs:documentation>
            </xs:annotation>
            <xs:complexType>
                <xs:attribute name="fileId" type="xs:NCName" use="required"/>
            </xs:complexType>
        </xs:element>
        <xs:element name="SparePartsAvailabilityYears" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Guaranteed years of spare parts availability after end of production</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:int">
                    <xs:minInclusive value="0"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="SpareParts" minOccurs="0">
            <xs:complexType>
                <xs:sequence>
                    <xs:element name="SparePart" maxOccurs="unbounded">
                        <xs:complexType>
                            <xs:sequence>
                                <xs:element name="Name" type="Locale"/>
                                <xs:element name="PartNumber" type="xs:string" minOccurs="0"/>
                                <xs:element name="GTIN" type="GTIN" minOccurs="0"/>
                                <xs:element name="AvailabilityYears" minOccurs="0">
                                    <xs:simpleType>
                                        <xs:restriction base="xs:int">
                                            <xs:minInclusive value="0"/>
                                        </xs:restriction>
                                    </xs:simpleType>
                                </xs:element>
                            </xs:sequence>
                        </xs:complexType>
                    </xs:element>
                </xs:sequence>
            </xs:complexType>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

#### 3.1.7 Compliance Type
```xml
<xs:complexType name="Compliance">
    <xs:annotation>
        <xs:documentation>Regulatory compliance declarations</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="RoHSCompliance" minOccurs="0">
            <xs:complexType>
                <xs:sequence>
                    <xs:element name="Compliant" type="xs:boolean"/>
                    <xs:element name="DeclarationReference" type="xs:anyURI" minOccurs="0"/>
                    <xs:element name="Exemptions" type="xs:string" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>List of applicable RoHS exemptions</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                </xs:sequence>
            </xs:complexType>
        </xs:element>
        <xs:element name="REACHCompliance" minOccurs="0">
            <xs:complexType>
                <xs:sequence>
                    <xs:element name="Compliant" type="xs:boolean"/>
                    <xs:element name="SCIPNumber" type="xs:string" minOccurs="0">
                        <xs:annotation>
                            <xs:documentation>SCIP database notification number</xs:documentation>
                        </xs:annotation>
                    </xs:element>
                    <xs:element name="DeclarationReference" type="xs:anyURI" minOccurs="0"/>
                </xs:sequence>
            </xs:complexType>
        </xs:element>
        <xs:element name="CEDeclaration" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Reference to CE Declaration of Conformity file</xs:documentation>
            </xs:annotation>
            <xs:complexType>
                <xs:attribute name="fileId" type="xs:NCName" use="required"/>
            </xs:complexType>
        </xs:element>
        <xs:element name="HazardousSubstances" minOccurs="0">
            <xs:complexType>
                <xs:sequence>
                    <xs:element name="Substance" maxOccurs="unbounded">
                        <xs:complexType>
                            <xs:sequence>
                                <xs:element name="Name" type="xs:string"/>
                                <xs:element name="CASNumber" type="xs:string" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>CAS Registry Number</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="ECNumber" type="xs:string" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>EC (EINECS/ELINCS) Number</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="Concentration" type="xs:double">
                                    <xs:annotation>
                                        <xs:documentation>Concentration in weight percent (0-100)</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="Location" type="xs:string" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>Location in the product (e.g., "LED driver", "housing")</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                                <xs:element name="SVHCListed" type="xs:boolean" minOccurs="0">
                                    <xs:annotation>
                                        <xs:documentation>Whether substance is on SVHC candidate list</xs:documentation>
                                    </xs:annotation>
                                </xs:element>
                            </xs:sequence>
                        </xs:complexType>
                    </xs:element>
                </xs:sequence>
            </xs:complexType>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

#### 3.1.8 Traceability Type
```xml
<xs:complexType name="Traceability">
    <xs:annotation>
        <xs:documentation>Manufacturing traceability information</xs:documentation>
    </xs:annotation>
    <xs:sequence>
        <xs:element name="CountryOfManufacture" minOccurs="0">
            <xs:annotation>
                <xs:documentation>ISO 3166-1 alpha-2 country code</xs:documentation>
            </xs:annotation>
            <xs:simpleType>
                <xs:restriction base="xs:string">
                    <xs:pattern value="[A-Z]{2}"/>
                </xs:restriction>
            </xs:simpleType>
        </xs:element>
        <xs:element name="ManufacturingPlant" type="xs:string" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Factory/plant identifier</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="ManufacturingDate" type="xs:date" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Production date</xs:documentation>
            </xs:annotation>
        </xs:element>
        <xs:element name="BatchNumber" type="xs:string" minOccurs="0">
            <xs:annotation>
                <xs:documentation>Production batch identifier</xs:documentation>
            </xs:annotation>
        </xs:element>
    </xs:sequence>
</xs:complexType>
```

---

### 3.2 Extensions to Existing Types

#### 3.2.1 Variant Extension
Add to `Variant` complexType sequence (after existing elements):

```xml
<!-- DPP Identification -->
<xs:element name="DigitalProductPassport" type="DigitalProductPassport" minOccurs="0">
    <xs:annotation>
        <xs:documentation>EU Digital Product Passport identification</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Environmental Impact -->
<xs:element name="CarbonFootprint" type="CarbonFootprint" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Carbon footprint and environmental impact data</xs:documentation>
    </xs:annotation>
</xs:element>

<xs:element name="WaterConsumption" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Manufacturing water consumption in liters</xs:documentation>
    </xs:annotation>
    <xs:simpleType>
        <xs:restriction base="xs:double">
            <xs:minInclusive value="0"/>
        </xs:restriction>
    </xs:simpleType>
</xs:element>

<!-- Material Composition -->
<xs:element name="MaterialComposition" type="MaterialComposition" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Material composition and recycled content</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Circularity -->
<xs:element name="Circularity" type="Circularity" minOccurs="0">
    <xs:annotation>
        <xs:documentation>End-of-life and circularity information</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Durability -->
<xs:element name="Durability" type="Durability" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Lifespan and warranty information</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Repairability -->
<xs:element name="Repairability" type="Repairability" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Repairability score and spare parts</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Compliance -->
<xs:element name="Compliance" type="Compliance" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Regulatory compliance (RoHS, REACH, CE)</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Traceability -->
<xs:element name="Traceability" type="Traceability" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Manufacturing traceability</xs:documentation>
    </xs:annotation>
</xs:element>

<!-- Luminaire Energy Label (supplement to LightSource EnergyLabels) -->
<xs:element name="LuminaireEnergyLabel" minOccurs="0">
    <xs:annotation>
        <xs:documentation>EU Energy Label for the complete luminaire</xs:documentation>
    </xs:annotation>
    <xs:simpleType>
        <xs:restriction base="xs:string">
            <xs:enumeration value="A"/>
            <xs:enumeration value="B"/>
            <xs:enumeration value="C"/>
            <xs:enumeration value="D"/>
            <xs:enumeration value="E"/>
            <xs:enumeration value="F"/>
            <xs:enumeration value="G"/>
        </xs:restriction>
    </xs:simpleType>
</xs:element>
```

#### 3.2.2 Header Extension
Add to `Header` complexType sequence:

```xml
<xs:element name="TakeBackProgram" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Manufacturer take-back program information</xs:documentation>
    </xs:annotation>
    <xs:complexType>
        <xs:sequence>
            <xs:element name="Available" type="xs:boolean"/>
            <xs:element name="ProgramURL" type="xs:anyURI" minOccurs="0"/>
            <xs:element name="Description" type="Locale" minOccurs="0"/>
        </xs:sequence>
    </xs:complexType>
</xs:element>
```

#### 3.2.3 ProductMetaData Extension
Add to `ProductMetaData` complexType sequence:

```xml
<xs:element name="Compliance" type="Compliance" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Global product compliance (can be overridden per Variant)</xs:documentation>
    </xs:annotation>
</xs:element>

<xs:element name="Repairability" type="Repairability" minOccurs="0">
    <xs:annotation>
        <xs:documentation>Global repairability information</xs:documentation>
    </xs:annotation>
</xs:element>
```

---

## 4. Priority Summary

### HIGH Priority (14 elements) - Required for DPP Compliance
| Element | Location | Type |
|---------|----------|------|
| DPPUniqueIdentifier | Variant | string |
| GTIN (luminaire) | Variant | GTIN (exists) |
| CarbonFootprint (total) | Variant | double |
| MaterialComposition | Variant | complex |
| TotalWeight | Variant | double |
| RecycledContentPercentage | Variant | double |
| RecyclabilityPercentage | Variant | double |
| ExpectedLifespanYears | Variant | int |
| WarrantyYears | Variant | int |
| RepairabilityScore | Variant | double |
| SparePartsAvailabilityYears | ProductMetaData | int |
| RoHSCompliance | ProductMetaData | complex |
| REACHCompliance | ProductMetaData | complex |
| HazardousSubstances | Variant | complex |

### MEDIUM Priority (12 elements) - Recommended
| Element | Location | Type |
|---------|----------|------|
| DataCarrierURI | Variant | anyURI |
| CarbonFootprintByPhase | Variant | complex |
| EPDReference | Variant | anyURI |
| DisassemblyTime | Variant | int |
| DisassemblyInstructions | Variant | fileRef |
| RecyclingInstructions | Variant | Locale |
| TakeBackProgram | Header | complex |
| RepairInstructions | ProductMetaData | fileRef |
| SpareParts | Variant | complex |
| CountryOfManufacture | Variant | string |
| CEDeclaration | ProductMetaData | fileRef |
| LuminaireEnergyLabel | Variant | enum |

### LOW Priority (3 elements) - Optional
| Element | Location | Type |
|---------|----------|------|
| WaterConsumption | Variant | double |
| ManufacturingPlant | Variant | string |
| ManufacturingDate | Variant | date |

---

## 5. Backwards Compatibility

All proposed elements are:
- **Optional** (`minOccurs="0"`)
- **Additive** (no existing elements modified or removed)
- **Non-breaking** (existing GLDF 1.0 files remain valid)

This allows:
- Gradual adoption by manufacturers
- Continued support for legacy files
- Phased implementation aligned with EU DPP rollout

---

## 6. Implementation Recommendations

### 6.1 Versioning Strategy
- **Option A:** GLDF 1.1.0 (minor version bump for backwards-compatible additions)
- **Option B:** GLDF 2.0.0 (major version for comprehensive DPP support)

**Recommendation:** GLDF 1.1.0 with optional DPP namespace extension

### 6.2 Validation
- Create separate XSD for DPP extension (gldf-dpp-1.0.xsd)
- Allow inclusion via `xs:import` for optional validation
- Provide JSON Schema equivalent for API integrations

### 6.3 Tooling Support
- Update GLDF Editor to support new elements
- Provide DPP data entry wizards
- Create validation tools for DPP completeness

---

## 7. References

- [EU Ecodesign for Sustainable Products Regulation (ESPR)](https://environment.ec.europa.eu/topics/circular-economy/ecodesign-sustainable-products-regulation_en)
- [Digital Product Passport Technical Standards](https://ec.europa.eu/info/law/better-regulation/)
- [ISO 14067:2018 - Carbon Footprint of Products](https://www.iso.org/standard/71206.html)
- [EU RoHS Directive 2011/65/EU](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32011L0065)
- [EU REACH Regulation (EC) No 1907/2006](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:02006R1907-20221217)
- [French Repairability Index](https://www.ecologie.gouv.fr/indice-reparabilite)

---

## 8. Change Request Summary

**Request:** Add EU Digital Product Passport (DPP) support to GLDF XSD

**Scope:**
- 8 new complex types
- 29 new elements across Variant, ProductMetaData, and Header
- All elements optional for backwards compatibility

**Timeline:**
- Proposal Review: Q1 2025
- Draft XSD: Q2 2025
- Public Comment: Q3 2025
- Final Release: Q4 2025 (ahead of 2026 DPP requirements)

**Contact:** [GLDF Working Group]

---

*Document prepared for GLDF Consortium review*
