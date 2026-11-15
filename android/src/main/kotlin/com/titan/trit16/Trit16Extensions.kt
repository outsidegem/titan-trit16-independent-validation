package com.titan.trit16

/**
 * Idiomatic Kotlin extensions and [Result]-returning strict wrappers over [Trit16] and [Kleene].
 */
public object Trit16Kotlin {
    public fun fromPlanes(pos: UShort, neg: UShort): Trit16 =
        Trit16.fromPlanes(pos.toInt() and 0xFFFF, neg.toInt() and 0xFFFF)

    public fun tryFromPlanesResult(pos: UShort, neg: UShort): Result<Trit16> =
        runCatching { Trit16.tryFromPlanes(pos.toInt() and 0xFFFF, neg.toInt() and 0xFFFF) }

    public fun fromU32Interleaved(interleaved: UInt): Trit16 =
        Trit16.fromU32Interleaved(interleaved.toInt())

    public fun tryFromU32InterleavedResult(interleaved: UInt): Result<Trit16> =
        runCatching { Trit16.tryFromU32Interleaved(interleaved.toInt()) }
}

public val Trit16.posUShort: UShort
    get() = pos().toUShort()

public val Trit16.negUShort: UShort
    get() = neg().toUShort()

public val Trit16.trapMaskUShort: UShort
    get() = trapMask().toUShort()

public operator fun Trit16.get(index: Int): Kleene = this.get(index)

public infix fun Trit16.tritAnd(other: Trit16): Trit16 = this.and(other)

public infix fun Trit16.tritOr(other: Trit16): Trit16 = this.or(other)

public fun Trit16.tritNot(): Trit16 = this.not()

public infix fun Trit16.tritCollide(other: Trit16): Trit16 = this.collide(other)

public fun Trit16.andStrictResult(other: Trit16): Result<Trit16> =
    runCatching { this.andStrict(other) }

public fun Trit16.orStrictResult(other: Trit16): Result<Trit16> =
    runCatching { this.orStrict(other) }

public fun Trit16.notStrictResult(): Result<Trit16> =
    runCatching { this.notStrict() }

public fun Trit16.collideStrictResult(other: Trit16): Result<Trit16> =
    runCatching { this.collideStrict(other) }
