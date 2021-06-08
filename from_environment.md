classDiagram
    Environment <|-- Factory
    Environment: <<trait>>
    Factory: <<trait>>
    Resource <|.. Service
    Resource: <<trait>>
    Resource: +type Config
    Resource: +type Customizer
    Service: <<trait>>
    FromEnvironment <|-- DescFromEnvironment
    FromEnvironment <|-- AutoDeriveFromEnvironment
    DescFromEnvironment <|-- PrefixedFromEnvironment
    FromEnvironment <|.. IsProperty
    IsProperty <|.. EnumProperty
    FromEnvironment: <<trait>>
    FromEnvironment: +from_env()
    DescFromEnvironment: <<trait>>
    DescFromEnvironment: +key_desc()
    PrefixedFromEnvironment: <<trait>>
    PrefixedFromEnvironment: +prefix()
    IsProperty: <<trait>>
    IsProperty: +from_property()
    IsProperty: +is_empty()
    EnumProperty: <<trait>>
    EnumProperty: +str_to_enum()